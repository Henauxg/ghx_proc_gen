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
        direction::{Cartesian2D, CoordinateSystem},
        GridData, GridDefinition,
    },
    GenerationError,
};

use self::{
    builder::{GeneratorBuilder, Unset},
    model::{ModelIndex, ModelInstance},
    node_heuristic::{InternalNodeSelectionHeuristic, NodeSelectionHeuristic},
    observer::GenerationUpdate,
    rules::Rules,
};

/// Defines a [`GeneratorBuilder`] used to create a generator
pub mod builder;
/// Defines [`crate::generator::model::Model`] and their associated type & utilities
pub mod model;
/// Defines the different possible [`NodeSelectionHeuristic`]
pub mod node_heuristic;
/// Defines different possible observers to view the results:execution of a [`Generator`]
pub mod observer;
/// Defines the [`Rules`] used by a [`Generator`]
pub mod rules;
/// Defines [`crate::generator::socket::Socket`] and their associated type & utilities
pub mod socket;

/// Defines a heuristic for the choice of a model among the possible ones when a node has been selected for generation.
pub enum ModelSelectionHeuristic {
    /// Choses a random model among the possible ones, weighted by each model weight.
    WeightedProbability,
}

/// Different ways to seed the RNG of the generator.
///
/// Note: No matter the selected mode, on each failed generation/reset, the generator will generate and use a new `u64` seed using the previous `u64` seed.
///
/// As an example: if a generation with 50 retries is requested with a seed `s1`, but the generations fails 14 times before finally succeeding with seed `s15`, requesting the generation with any of the seeds `s1`, `s2`, ... to `s15` will give the exact same final successful result. However, while `s1` will need to redo the 14 failed generations before succeeding,`s15` will directly generate the successfull result.
pub enum RngMode {
    /// The generator will use the given seed for its random source.
    ///
    Seeded(u64),
    /// The generator will use a random seed for its random source.
    ///
    /// The randomly generated seed can still be retrieved by calling `get_seed` on the generator once created.
    RandomSeed,
}

/// Represents the current generation state, if not failed.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum GenerationStatus {
    /// The generation has not ended yet.
    Ongoing,
    /// The generation ended succesfully. The whole grid is generated.
    Done,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

/// Output of a [`Generator`] in the context of its [`crate::grid::GridDefinition`].
#[derive(Clone, Copy, Debug)]
pub struct GridNode {
    /// Index of the node in the [`crate::grid::GridDefinition`]
    pub node_index: usize,
    /// Generated node data
    pub model_instance: ModelInstance,
}

struct PropagationEntry {
    node_index: usize,
    model_index: ModelIndex,
}

/// Model synthesis/WFC generator.
/// Use a [`GeneratorBuilder`] to get an instance of a [`Generator`].
pub struct Generator<T: CoordinateSystem + Clone> {
    // === Read-only configuration ===
    grid: GridDefinition<T>,
    rules: Arc<Rules<T>>,
    max_retry_count: u32,

    // === Generation state ===
    seed: u64,
    rng: StdRng,
    status: InternalGeneratorStatus,
    /// `nodes[node_index * self.rules.models_count() + model_index]` is true (1) if model with index `model_index` is still allowed on node with index `node_index`
    nodes: BitVec<usize>,
    /// Stores how many models are still possible for a given node
    possible_models_counts: Vec<usize>,
    node_selection_heuristic: InternalNodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    /// Observers signaled with updates of the nodes.
    observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,

    // === Constraint satisfaction algorithm data ===
    /// Stack of bans to propagate
    propagation_stack: Vec<PropagationEntry>,
    /// The value at `support_count[node_index][model_index][direction]` represents the number of supports of a `model_index` at `node_index` from `direction`
    supports_count: Array<usize, Ix3>,
}

impl<T: CoordinateSystem + Clone> Generator<T> {
    /// Returns a new `GeneratorBuilder`
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

        let node_selection_heuristic = InternalNodeSelectionHeuristic::from_external(
            node_selection_heuristic,
            &rules,
            grid.total_size(),
        );

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
            possible_models_counts: vec![models_count; nodes_count],
            observers: Vec::new(),

            propagation_stack: Vec::new(),
            supports_count: Array::zeros((nodes_count, models_count, direction_count)),
        };
        // We don't do `initialize_supports_count` yet since it may generate some node(s).
        generator
    }

    /// Returns the seed that was used to initialize the generator RNG for this generation. See [`RngMode`] for more information.
    pub fn get_seed(&self) -> u64 {
        self.seed
    }

    /// Returns the [`GridDefinition`] used by the generator
    pub fn grid(&self) -> &GridDefinition<T> {
        &self.grid
    }

    /// Tries to generate the whole grid. If the generation fails due to a contradiction, it will retry `max_retry_count` times before returning the last encountered [`GenerationError`]
    ///
    /// If the generation has ended (successful or not), calling `generate` will reinitialize the generator before starting the generation.
    /// If the generation was already started by previous calls to `select_and_propagate`, this will simply continue the generation.
    pub fn generate_collected(&mut self) -> Result<GridData<T, ModelInstance>, GenerationError> {
        self.generate()?;
        Ok(self.to_grid_data())
    }

    /// Same as `generate_collected` but does not return a filled [`GridData`] when the generation is done. You can still retrieve a filled [`GridData`] by calling the `to_grid_data` function.
    ///
    /// This can be usefull if you retrieve the data via other means such as observers.
    pub fn generate(&mut self) -> Result<(), GenerationError> {
        for _i in 1..self.max_retry_count {
            #[cfg(feature = "debug-traces")]
            info!("Try n°{}", _i);

            match self.internal_generate() {
                Ok(_) => return Ok(()),
                Err(_) => (),
            }
        }
        #[cfg(feature = "debug-traces")]
        info!("Try n°{}", self.max_retry_count + 1);
        self.internal_generate()
    }

    /// Advances the generation by one "step": select a node and a model and propagate the changes.
    ///
    /// Returns the [`GenerationStatus`] if the step executed successfully and [`crate::GenerationError`] if the generation fails due to a contradiction.
    ///
    /// If the generation has ended (successfully or not), calling `select_and_propagate` again will reinitialize the [`Generator`] before starting a new generation.
    ///
    /// **Note**: One call to `select_and_propagate` **can** lead to more than one node generated if the propagation phase forces some other node(s) into a definite state (due to only one possible model remaining on a node)
    pub fn select_and_propagate(&mut self) -> Result<GenerationStatus, GenerationError> {
        self.internal_select_and_propagate(&mut None)
    }

    /// Same as `select_and_propagate` but collects and return the generated [`GridNode`] when successful.
    pub fn select_and_propagate_collected(
        &mut self,
    ) -> Result<(GenerationStatus, Vec<GridNode>), GenerationError> {
        let mut collector = Some(Vec::new());
        let res = self.internal_select_and_propagate(&mut collector)?;
        // We know that collector is Some.
        Ok((res, collector.unwrap()))
    }

    fn reinitialize(
        &mut self,
        collector: &mut Option<Vec<GridNode>>,
    ) -> Result<(), GenerationError> {
        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Reinitializing(self.seed));
        }

        self.seed = self.rng.gen::<u64>();
        self.rng = StdRng::seed_from_u64(self.seed);

        #[cfg(feature = "debug-traces")]
        info!(
            "Reinitializing generator with seed {}, state was {:?}",
            self.seed, self.status
        );

        self.node_selection_heuristic.reinitialize();
        self.status = InternalGeneratorStatus::Ongoing;
        self.nodes = bitvec![1;self.rules.models_count() * self.grid.total_size() ];
        self.possible_models_counts = vec![self.rules.models_count(); self.grid.total_size()];
        self.propagation_stack = Vec::new();
        self.initialize_supports_count(collector)?;

        Ok(())
    }

    /// Initialize the supports counts array. This may already start to generate/ban/... some nodes according to the given constraints.
    ///
    /// Returns `Ok` if the initialization went well and sets the internal status to [`InternalGeneratorStatus::Ongoing`]. Else, sets the internal status to [`InternalGeneratorStatus::Failed`] and return [`ProcGenError::GenerationFailure`]
    fn initialize_supports_count(
        &mut self,
        collector: &mut Option<Vec<GridNode>>,
    ) -> Result<(), GenerationError> {
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
                                if let Err(err) = self.ban_model_from_node(node, model, collector) {
                                    self.signal_contradiction(node);
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
        if let Err(err) = self.propagate(collector) {
            self.signal_contradiction(err.node_index);
            return Err(err);
        };

        #[cfg(feature = "debug-traces")]
        debug!("Support counts initialized successfully");

        self.status = InternalGeneratorStatus::Ongoing;

        Ok(())
    }

    fn internal_generate(&mut self) -> Result<(), GenerationError> {
        match self.status {
            InternalGeneratorStatus::Init => self.initialize_supports_count(&mut None)?,
            InternalGeneratorStatus::Ongoing => (),
            InternalGeneratorStatus::Done | InternalGeneratorStatus::Failed => {
                self.reinitialize(&mut None)?
            }
        };

        // Grid total size is an upper limit to the number of iterations. We avoid an unnecessary while loop.
        for _i in 0..self.grid.total_size() {
            match self.internal_select_and_propagate(&mut None) {
                Ok(GenerationStatus::Done) => break,
                Ok(GenerationStatus::Ongoing) => (),
                Err(e) => return Err(e),
            };
        }
        Ok(())
    }

    fn internal_select_and_propagate(
        &mut self,
        collector: &mut Option<Vec<GridNode>>,
    ) -> Result<GenerationStatus, GenerationError> {
        match self.status {
            InternalGeneratorStatus::Init => self.initialize_supports_count(collector)?,
            InternalGeneratorStatus::Ongoing => (),
            InternalGeneratorStatus::Done | InternalGeneratorStatus::Failed => {
                self.reinitialize(collector)?
            }
        };

        let node_index = match self
            .node_selection_heuristic
            .select_node(&self.possible_models_counts, &mut self.rng)
        {
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
            self.rules.model(selected_model_index),
            node_index,
            self.grid.get_position(node_index)
        );
        if !self.observers.is_empty() || collector.is_some() {
            self.signal_selection(collector, node_index, selected_model_index);
        }

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
        self.possible_models_counts[node_index] = 1;

        if let Err(err) = self.propagate(collector) {
            self.signal_contradiction(err.node_index);
            return Err(err);
        };

        Ok(GenerationStatus::Ongoing)
    }

    fn signal_selection(
        &mut self,
        collector: &mut Option<Vec<GridNode>>,
        node_index: usize,
        model_index: ModelIndex,
    ) {
        let grid_node = GridNode {
            node_index,
            model_instance: self.rules.model(model_index).to_instance(),
        };
        let update = GenerationUpdate::Generated(grid_node);
        for obs in &mut self.observers {
            let _ = obs.send(update);
        }
        if let Some(collector) = collector {
            collector.push(grid_node);
        }
    }

    /// Returns [`ProcGenError::GenerationFailure`] if a node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    fn propagate(&mut self, collector: &mut Option<Vec<GridNode>>) -> Result<(), GenerationError> {
        // Clone the ref to allow for mutability of other members in the interior loops
        let rules = Arc::clone(&self.rules);

        while let Some(from) = self.propagation_stack.pop() {
            let from_position = self.grid.get_position(from.node_index);

            #[cfg(feature = "debug-traces")]
            trace!(
                "Propagate removal of model {} for node {}",
                self.rules.model(from.model_index),
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
                                self.ban_model_from_node(to_node_index, model, collector)?;
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
            self.rules.model(model_index),
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
    fn ban_model_from_node(
        &mut self,
        node_index: usize,
        model: usize,
        collector: &mut Option<Vec<GridNode>>,
    ) -> Result<(), GenerationError> {
        // Update the supports
        for dir in self.grid.directions() {
            let supports_count = &mut self.supports_count[(node_index, model, *dir as usize)];
            *supports_count = 0;
        }
        // Update the state
        self.nodes
            .set(node_index * self.rules.models_count() + model, false);

        let number_of_models_left = &mut self.possible_models_counts[node_index];
        *number_of_models_left = number_of_models_left.saturating_sub(1);

        self.node_selection_heuristic
            .handle_ban(node_index, model, self.rules.weight(model));

        #[cfg(feature = "debug-traces")]
        trace!(
            "Ban model {} from node {} at position {:?}, {} models left",
            self.rules.model(model),
            node_index,
            self.grid.get_position(node_index),
            number_of_models_left
        );

        match *number_of_models_left {
            0 => return Err(GenerationError { node_index }),
            1 => {
                #[cfg(feature = "debug-traces")]
                {
                    let forced_model = self.get_model_index(node_index);
                    debug!(
                        "Previous bans force model {} for node {} at position {:?}",
                        self.rules.model(forced_model),
                        node_index,
                        self.grid.get_position(node_index)
                    );
                }

                // Check beforehand to avoid `get_model_index` call
                if !self.observers.is_empty() || collector.is_some() {
                    self.signal_selection(collector, node_index, self.get_model_index(node_index));
                }
            }
            _ => (),
        }

        // Enqueue removal for propagation
        self.enqueue_removal_to_propagate(node_index, model);

        Ok(())
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
    fn to_grid_data(&self) -> GridData<T, ModelInstance> {
        let mut generated_nodes = Vec::with_capacity(self.nodes.len());
        for node_index in 0..self.grid.total_size() {
            let model_index = self.get_model_index(node_index);
            let expanded_model = self.rules.model(model_index);
            generated_nodes.push(expanded_model.to_instance())
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

    fn signal_contradiction(&mut self, node_index: usize) {
        #[cfg(feature = "debug-traces")]
        debug!("Generation failed due to a contradiction");

        self.status = InternalGeneratorStatus::Failed;
        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Failed(node_index));
        }
    }
}
