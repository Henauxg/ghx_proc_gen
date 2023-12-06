use bitvec::{bitvec, vec::BitVec};
use ndarray::{Array, Ix3};
use rand::{
    distributions::Distribution, distributions::WeightedIndex, rngs::StdRng, Rng, SeedableRng,
};
use std::sync::Arc;

#[cfg(feature = "debug-traces")]
use tracing::{debug, info, trace};

use crate::{
    grid::{
        direction::{Cartesian2D, DirectionSet},
        GridData, GridDefinition,
    },
    ProcGenError,
};

use self::{
    builder::{GeneratorBuilder, Unset},
    node::{GeneratedNode, ModelIndex},
    observer::GenerationUpdate,
    rules::Rules,
};

pub mod builder;
pub mod node;
pub mod observer;
pub mod rules;

const MAX_NOISE_VALUE: f32 = 1E-2;

/// Defines a heuristic for the choice of a node to generate.
pub enum NodeSelectionHeuristic {
    /// The node with with the minimum count of possible models remaining will be chosen at each selection iteration. If multiple nodes have the same value, a random one is picked.
    MinimumRemainingValue,
    /// A random node with no special features (except not being generated yet) will be chosen at each selection iteration.
    Random,
}

/// Defines a heuristic for the choice of a model among the possible ones when a node has been selected for generation.
pub enum ModelSelectionHeuristic {
    /// Choses a random model among the possible ones, weighted by each model weight.
    WeightedProbability,
}

pub enum RngMode {
    /// The generator will use the given seed for its random source.
    Seeded(u64),
    /// The generator will use a random seed for its random source. The randomly generated seed can be retrieved by calling `get_seed` on the generator.
    RandomSeed,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum GenerationStatus {
    /// The generation has not ended yet.
    Ongoing,
    /// The generation ended succesfully. The whole grid is generated.
    Done,
}

#[derive(Debug, Clone, Copy)]
enum InternalGeneratorStatus {
    /// Initial state.
    Init,
    /// Generation has not finished.
    Ongoing,
    /// Generation ended succesfully.
    Done,
    /// Generation failed due to a contradiction.
    Failed,
}

struct PropagationEntry {
    node_index: usize,
    model_index: ModelIndex,
}

/// Model synthesis/WFC generator.
/// Use a [`GeneratorBuilder`] to get an instance of a [`Generator`].
pub struct Generator<T: DirectionSet + Clone> {
    // === Read-only configuration ===
    grid: GridDefinition<T>,
    rules: Arc<Rules<T>>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,

    // === Internal ===
    rng: StdRng,
    seed: u64,

    // === Generation state ===
    status: InternalGeneratorStatus,
    /// `nodes[node_index * self.rules.models_count() + model_index]` is true (1) if model with index `model_index` is still allowed on node with index `node_index`
    nodes: BitVec<usize>,
    /// Stores how many models are still possible for a given node
    possible_models_count: Vec<usize>,
    /// Vector of observers currently being signaled with updates of the nodes.
    observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,

    // === Constraint satisfaction algorithm data ===
    /// Stack of bans to propagate
    propagation_stack: Vec<PropagationEntry>,
    /// The value at `support_count[node_index][model_index][direction]` represents the number of supports of a `model_index` at `node_index` from `direction`
    supports_count: Array<usize, Ix3>,
}

impl<T: DirectionSet + Clone> Generator<T> {
    pub fn builder() -> GeneratorBuilder<Unset, Unset, Cartesian2D> {
        GeneratorBuilder::new()
    }

    fn new(
        rules: Arc<Rules<T>>,
        grid: GridDefinition<T>,
        max_retry_count: u32,
        node_selection_heuristic: NodeSelectionHeuristic,
        model_selection_heuristic: ModelSelectionHeuristic,
        rng_mode: RngMode,
    ) -> Self {
        let models_count = rules.models_count();
        let nodes_count = grid.total_size();
        let direction_count = grid.directions().len();

        let seed = match rng_mode {
            RngMode::Seeded(seed) => seed,
            RngMode::RandomSeed => rand::thread_rng().gen::<u64>(),
        };

        let generator = Self {
            grid,
            rules,
            max_retry_count,
            node_selection_heuristic,
            model_selection_heuristic,

            rng: StdRng::seed_from_u64(seed),
            seed,

            status: InternalGeneratorStatus::Init,
            nodes: bitvec![1; nodes_count * models_count],
            possible_models_count: vec![models_count; nodes_count],
            observers: Vec::new(),

            propagation_stack: Vec::new(),
            supports_count: Array::zeros((nodes_count, models_count, direction_count)),
        };
        // We don't do `initialize_supports_count` yet since it may generate some node(s).
        generator
    }

    /// Returns the seed that was used to initialize the generator RNG.
    pub fn get_seed(&self) -> u64 {
        self.seed
    }

    pub fn grid(&self) -> &GridDefinition<T> {
        &self.grid
    }

    fn reinitialize(&mut self) -> Result<(), ProcGenError> {
        #[cfg(feature = "debug-traces")]
        info!("Reinitializing generator, state was {:?}", self.status);

        self.status = InternalGeneratorStatus::Ongoing;
        self.nodes = bitvec![1;self.rules.models_count() * self.grid.total_size() ];
        self.possible_models_count = vec![self.rules.models_count(); self.grid.total_size()];
        self.propagation_stack = Vec::new();
        self.initialize_supports_count()?;

        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Reinitialized);
        }
        Ok(())
    }

    /// Initialize the supports counts array. This may already start to generate/ban/... some nodes according to the given constraints.
    ///
    /// Returns `Ok` if the initialization went well and sets the internal status to [`InternalGeneratorStatus::Ongoing`]. Else, sets the internal status to [`InternalGeneratorStatus::Failed`] and return [`ProcGenError::GenerationFailure`]
    fn initialize_supports_count(&mut self) -> Result<(), ProcGenError> {
        #[cfg(feature = "debug-traces")]
        debug!("Initializing support counts");

        let mut neighbours = vec![None; self.grid.directions().len()];
        for node in 0..self.grid.total_size() {
            // For a given `node`, `neighbours[direction]` will hold the optionnal index of the neighbour node in `direction`
            for direction in self.grid.directions() {
                let grid_pos = self.grid.get_position(node);
                neighbours[*direction as usize] = self.grid.get_next_index(&grid_pos, *direction);
            }

            for model in 0..self.rules.models_count() {
                for direction in self.grid.directions() {
                    let opposite_dir = direction.opposite();
                    // During initialization, the support count for a model "from" a direction is simply the count of allowed adjacent models when looking in the opposite direction, or 0 for a non-looping border (no neighbour from this direction).
                    match neighbours[opposite_dir as usize] {
                        Some(_) => {
                            let allowed_models_count =
                                self.rules.allowed_models(model, opposite_dir).len();
                            self.supports_count[(node, model, *direction as usize)] =
                                allowed_models_count;
                            if allowed_models_count == 0 && self.is_model_possible(node, model) {
                                // Ban model for node since it would 100% lead to a contradiction at some point during the generation.
                                if let Err(err) = self.ban_model_from_node(node, model) {
                                    self.signal_contradiction();
                                    return Err(err);
                                }
                                // We don't need to process the remaining directions, iterate on the next model.
                                break;
                            }
                        }
                        None => self.supports_count[(node, model, *direction as usize)] = 0,
                    };
                }
            }
        }

        // Propagate the potential bans that occurred during initialization
        if let Err(err) = self.propagate() {
            self.signal_contradiction();
            return Err(err);
        };

        #[cfg(feature = "debug-traces")]
        debug!("Support counts initialized successfully");

        self.status = InternalGeneratorStatus::Ongoing;

        Ok(())
    }

    /// Tries to generate the whole grid. If the generation fails due to a contradiction, it will retry `max_retry_count` times before returning `ProcGenError::GenerationFailure`
    ///
    /// If the generation has ended (successful or not), calling `generate` will reinitialize the generator before starting the generation.
    /// If the generation was already started by previous calls to [`Generator::select_and_propagate`], this will simply continue the generation.
    pub fn generate(&mut self) -> Result<GridData<T, GeneratedNode>, ProcGenError> {
        self.generate_without_output()?;
        Ok(self.to_grid_data())
    }

    /// Same as [`generate`] but does dot return a filled [`GridData`] when the generation is done. You can still retrieve a filled [`GridData`] by calling [`to_grid_data`].
    ///
    /// This can be usefull if you retrieve the data via other means (observers, ...)
    pub fn generate_without_output(&mut self) -> Result<(), ProcGenError> {
        for _i in 1..self.max_retry_count + 1 {
            #[cfg(feature = "debug-traces")]
            info!("Try n°{}", _i);

            match self.status {
                InternalGeneratorStatus::Init => self.initialize_supports_count()?,
                InternalGeneratorStatus::Ongoing => (),
                InternalGeneratorStatus::Done | InternalGeneratorStatus::Failed => {
                    self.reinitialize()?
                }
            };

            // TODO Split generation in multiple blocks
            match self.generate_all_nodes() {
                Ok(_) => return Ok(()),
                Err(_) => {
                    self.status = InternalGeneratorStatus::Failed; // Should already be set by callee.
                }
            }
        }
        Err(ProcGenError::GenerationFailure)
    }

    fn generate_all_nodes(&mut self) -> Result<(), ProcGenError> {
        // Grid total size is an upper limit to the number of iterations. We avoid an unnecessary while loop.
        for _i in 0..self.grid.total_size() {
            match self.select_and_propagate() {
                Ok(GenerationStatus::Done) => break,
                Ok(GenerationStatus::Ongoing) => (),
                Err(e) => return Err(e),
            };
        }
        Ok(())
    }

    /// Advances the generation by one "step". Returns the [`GenerationStatus`] if the step executed successfully and [`ProcGenError::GenerationFailure`] if the generation fails due to a contradiction.
    ///
    /// If the generation has ended (successfully or not), calling `select_and_propagate` again will reinitialize the [`Generator`] before starting a new generation.
    ///
    /// **Note**: One call to `select_and_propagate` can lead to more than 1 node generated if the propagation phase forces some other node(s) into a definite state (due to only 1 possible model remaining on a node)
    pub fn select_and_propagate(&mut self) -> Result<GenerationStatus, ProcGenError> {
        match self.status {
            InternalGeneratorStatus::Init => self.initialize_supports_count()?,
            InternalGeneratorStatus::Ongoing => (),
            InternalGeneratorStatus::Done | InternalGeneratorStatus::Failed => {
                self.reinitialize()?
            }
        };

        let node_index = match self.select_node_to_generate() {
            Some(index) => index,
            None => {
                self.status = InternalGeneratorStatus::Done;
                return Ok(GenerationStatus::Done);
            }
        };
        // We found a node not yet generated. "Observe/collapse" the node: select a model for the node
        let selected_model_index = self.select_model(node_index);

        #[cfg(feature = "debug-traces")]
        debug!(
            "Heuristics selected model {} for node {} at position {:?}",
            selected_model_index,
            node_index,
            self.grid.get_position(node_index)
        );
        self.signal_selection_to_observers(node_index, selected_model_index);

        // Iterate all the possible models because we don't have an easy way to iterate only the models possible at node_index. But we'll filter impossible models right away. TODO: iter_ones ?
        for model_index in 0..self.rules.models_count() {
            if model_index == selected_model_index {
                continue;
            }
            if !self.is_model_possible(node_index, model_index) {
                continue;
            }

            // Enqueue removal for propagation
            self.enqueue_removal_to_propagate(node_index, model_index);

            // None of these model are possible on this node now, set their support to 0
            for dir in self.grid.directions() {
                let supports_count =
                    &mut self.supports_count[(node_index, model_index, *dir as usize)];
                *supports_count = 0;
            }
        }
        // Remove eliminated possibilities (after enqueuing the propagation entries because we currently filter on the possible models)
        // TODO Remove alias ?
        let models_count = self.rules.models_count();
        for mut bit in self.nodes
            [node_index * models_count..node_index * models_count + models_count]
            .iter_mut()
        {
            *bit = false;
        }
        self.nodes
            .set(node_index * models_count + selected_model_index, true);
        self.possible_models_count[node_index] = 1;

        if let Err(err) = self.propagate() {
            self.signal_contradiction();
            return Err(err);
        };

        Ok(GenerationStatus::Ongoing)
    }

    fn signal_selection_to_observers(&mut self, node_index: usize, model_index: ModelIndex) {
        let update = GenerationUpdate::Generated {
            node_index,
            generated_node: self.rules.model(model_index).to_generated(),
        };
        for obs in &mut self.observers {
            let _ = obs.send(update);
        }
    }

    /// Returns [`ProcGenError::GenerationFailure`] if a node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    fn propagate(&mut self) -> Result<(), ProcGenError> {
        // Clone the ref to allow for mutability of other members in the interior loops
        let rules = Arc::clone(&self.rules);

        while let Some(from) = self.propagation_stack.pop() {
            let from_position = self.grid.get_position(from.node_index);

            #[cfg(feature = "debug-traces")]
            trace!(
                "Propagate removal of model {} for node {}",
                from.model_index,
                from.node_index
            );

            // We want to update all the adjacent nodes (= in all directions)
            for dir in self.grid.directions() {
                // Get the adjacent node in this direction, it may not exist.
                if let Some(to_node_index) = self.grid.get_next_index(&from_position, *dir) {
                    // Decrease the support count of all models previously supported by "from"
                    for &model in rules.allowed_models(from.model_index, *dir) {
                        let supports_count =
                            &mut self.supports_count[(to_node_index, model, *dir as usize)];
                        if *supports_count > 0 {
                            *supports_count -= 1;
                            // When we find a model which is now unsupported, we queue a ban
                            // We check > 0  and for == because we only want to queue the event once.
                            if *supports_count == 0 {
                                self.ban_model_from_node(to_node_index, model)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn enqueue_removal_to_propagate(&mut self, node_index: usize, model_index: ModelIndex) {
        #[cfg(feature = "debug-traces")]
        trace!(
            "Enqueue removal for propagation: model {} from node {}",
            model_index,
            node_index
        );
        self.propagation_stack.push(PropagationEntry {
            node_index,
            model_index,
        });
    }

    /// Returns [`ProcGenError::GenerationFailure`] if the node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    ///
    /// Should only be called a model that is still possible for this node
    fn ban_model_from_node(&mut self, node: usize, model: usize) -> Result<(), ProcGenError> {
        // Update the supports
        for dir in self.grid.directions() {
            let supports_count = &mut self.supports_count[(node, model, *dir as usize)];
            *supports_count = 0;
        }
        // Update the state
        self.nodes
            .set(node * self.rules.models_count() + model, false);

        let number_of_models_left = &mut self.possible_models_count[node];
        *number_of_models_left = number_of_models_left.saturating_sub(1);

        #[cfg(feature = "debug-traces")]
        trace!(
            "Ban model {} from node {} at position {:?}, {} models left",
            model,
            node,
            self.grid.get_position(node),
            number_of_models_left
        );

        match *number_of_models_left {
            0 => return Err(ProcGenError::GenerationFailure),
            1 => {
                #[cfg(feature = "debug-traces")]
                debug!(
                    "Previous bans force model {} for node {} at position {:?}",
                    model,
                    node,
                    self.grid.get_position(node)
                );
                self.signal_selection_to_observers(node, self.get_model_index(node));
            }
            _ => (),
        }

        // Enqueue removal for propagation
        self.enqueue_removal_to_propagate(node, model);

        Ok(())
    }

    fn select_node_to_generate<'a>(&mut self) -> Option<usize> {
        // Pick a node according to the heuristic
        match self.node_selection_heuristic {
            NodeSelectionHeuristic::MinimumRemainingValue => {
                let mut min = f32::MAX;
                let mut picked_node = None;
                for (index, &count) in self.possible_models_count.iter().enumerate() {
                    // If the node is not generated yet (multiple possibilities)
                    if count > 1 {
                        // Noise added to entropy so that when evaluating multiples candidates with the same value, we pick a random one, not in the evaluating order.
                        let noise = MAX_NOISE_VALUE * self.rng.gen::<f32>();
                        if (count as f32 + noise) < min {
                            min = count as f32 + noise;
                            picked_node = Some(index);
                        }
                    }
                }
                picked_node
            }
            NodeSelectionHeuristic::Random => {
                let mut picked_node = None;
                let mut candidates = Vec::new();
                for (index, &count) in self.possible_models_count.iter().enumerate() {
                    if count > 1 {
                        candidates.push(index);
                    }
                }
                if candidates.len() > 0 {
                    picked_node = Some(candidates[self.rng.gen_range(0..candidates.len())]);
                }
                picked_node
            }
        }
    }

    /// There should at least be one possible model for this node index. May panic otherwise.
    fn select_model(&mut self, node_index: usize) -> usize {
        match self.model_selection_heuristic {
            ModelSelectionHeuristic::WeightedProbability => {
                let possible_models: Vec<ModelIndex> = (0..self.rules.models_count())
                    .filter(|&model_index| self.is_model_possible(node_index, model_index))
                    .collect();

                // TODO May cache the current sum of weights at each node.
                let weighted_distribution = WeightedIndex::new(
                    possible_models
                        .iter()
                        .map(|&model_index| self.rules.weight(model_index)),
                )
                .unwrap();
                possible_models[weighted_distribution.sample(&mut self.rng)]
            }
        }
    }

    #[inline]
    fn is_model_possible(&self, node: usize, model: usize) -> bool {
        self.nodes[node * self.rules.models_count() + model] == true
    }

    /// Should only be called when the nodes are fully generated
    fn to_grid_data(&self) -> GridData<T, GeneratedNode> {
        let mut generated_nodes = Vec::with_capacity(self.nodes.len());
        for node_index in 0..self.grid.total_size() {
            let model_index = self.get_model_index(node_index);
            let expanded_model = self.rules.model(model_index);
            generated_nodes.push(expanded_model.to_generated())
        }

        GridData::new(self.grid.clone(), generated_nodes)
    }

    fn get_model_index(&self, node_index: usize) -> usize {
        self.nodes[node_index * self.rules.models_count()
            ..node_index * self.rules.models_count() + self.rules.models_count()]
            .first_one()
            .unwrap_or(0)
    }

    fn add_observer_queue(&mut self) -> crossbeam_channel::Receiver<GenerationUpdate> {
        // We can't simply bound to the number of nodes since we might retry some generations. (and send more than number_of_nodes updates)
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        receiver
    }

    fn signal_contradiction(&mut self) {
        #[cfg(feature = "debug-traces")]
        debug!("Generation failed due to a contradiction");

        self.status = InternalGeneratorStatus::Failed;
        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Failed);
        }
    }
}
