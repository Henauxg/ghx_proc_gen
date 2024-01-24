use bitvec::{bitvec, vec::BitVec};
use ndarray::{Array, Ix3};
use rand::{
    distributions::Distribution, distributions::WeightedIndex, rngs::StdRng, Rng, SeedableRng,
};
use std::sync::Arc;

#[cfg(feature = "debug-traces")]
use tracing::{debug, info, trace};

#[cfg(feature = "bevy")]
use bevy::ecs::component::Component;

use crate::{
    grid::{
        direction::{Cartesian2D, CoordinateSystem},
        GridData, GridDefinition, NodeIndex,
    },
    GenerationError, NodeSetError,
};

use self::{
    builder::{GeneratorBuilder, Unset},
    model::{ModelInstance, ModelVariantIndex},
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
    /// The randomly generated seed can still be retrieved on the generator once created.
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

#[derive(Debug, Clone, Copy)]
enum InternalGeneratorStatus {
    /// Generation has not finished.
    Ongoing,
    /// Generation ended succesfully.
    Done,
    /// Generation failed due to a contradiction.
    Failed(GenerationError),
}

/// Output of a [`Generator`] in the context of its [`crate::grid::GridDefinition`].
#[derive(Clone, Copy, Debug)]
pub struct GridNode {
    /// Index of the node in the [`crate::grid::GridDefinition`]
    pub node_index: usize,
    /// Generated node data
    pub model_instance: ModelInstance,
}

pub struct GenInfo {
    pub try_count: u32,
}

struct PropagationEntry {
    node_index: usize,
    model_index: ModelVariantIndex,
}

enum NodeSetStatus {
    AlreadySet,
    CanBeSet,
}

type Collector<'a> = Option<&'a mut Vec<GridNode>>;

/// Model synthesis/WFC generator.
/// Use a [`GeneratorBuilder`] to get an instance of a [`Generator`].
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Generator<T: CoordinateSystem> {
    // === Read-only configuration ===
    grid: GridDefinition<T>,
    rules: Arc<Rules<T>>,
    initial_nodes: Arc<Vec<(NodeIndex, ModelVariantIndex)>>,

    // === Configuration ===
    max_retry_count: u32,

    // === Generation state ===
    seed: u64,
    rng: StdRng,
    status: InternalGeneratorStatus,
    /// `nodes[node_index * self.rules.models_count() + model_index]` is true (1) if model with index `model_index` is still allowed on node with index `node_index`
    nodes: BitVec<usize>,
    nodes_left_to_generate: usize,
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

impl<T: CoordinateSystem> Generator<T> {
    /// Returns a new `GeneratorBuilder`
    pub fn builder() -> GeneratorBuilder<Unset, Unset, Cartesian2D> {
        GeneratorBuilder::new()
    }

    fn create(
        rules: Arc<Rules<T>>,
        grid: GridDefinition<T>,
        initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,
        max_retry_count: u32,
        node_selection_heuristic: NodeSelectionHeuristic,
        model_selection_heuristic: ModelSelectionHeuristic,
        rng_mode: RngMode,
        observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,
        collector: &mut Collector,
    ) -> Result<Self, NodeSetError> {
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

        let mut generator = Self {
            grid,
            rules,
            initial_nodes: Arc::new(initial_nodes),

            max_retry_count,

            node_selection_heuristic,
            model_selection_heuristic,

            rng: StdRng::seed_from_u64(seed),
            seed,

            status: InternalGeneratorStatus::Ongoing,
            nodes: bitvec![1; nodes_count * models_count],
            nodes_left_to_generate: nodes_count,
            possible_models_counts: vec![models_count; nodes_count],

            observers,

            propagation_stack: Vec::new(),
            supports_count: Array::zeros((nodes_count, models_count, direction_count)),
        };
        match generator.pregen(collector) {
            Ok(_status) => Ok(generator),
            Err(err) => Err(err),
        }
    }

    pub fn max_retry_count(&self) -> u32 {
        self.max_retry_count
    }

    pub fn set_max_retry_count(&mut self, max_retry_count: u32) {
        self.max_retry_count = max_retry_count;
    }

    /// Returns the seed that was used to initialize the generator RNG for this generation. See [`RngMode`] for more information.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns the [`GridDefinition`] used by the generator
    pub fn grid(&self) -> &GridDefinition<T> {
        &self.grid
    }

    /// Returns the [`Rules`] used by the generator
    pub fn rules(&self) -> &Rules<T> {
        &self.rules
    }

    /// Returns how many nodes are left to generate
    pub fn nodes_left(&self) -> usize {
        self.nodes_left_to_generate
    }

    pub fn to_grid_data(&self) -> Option<GridData<T, ModelInstance>> {
        match self.status {
            InternalGeneratorStatus::Ongoing => None,
            InternalGeneratorStatus::Failed(_) => None,
            InternalGeneratorStatus::Done => Some(self.internal_to_grid_data()),
        }
    }

    /// Tries to generate the whole grid. If the generation fails due to a contradiction, it will retry `max_retry_count` times before returning the last encountered [`GenerationError`]
    ///
    /// If the generation has ended (successful or not), calling `generate` will reinitialize the generator before starting the generation.
    /// If the generation was already started by previous calls to `select_and_propagate`, this will simply continue the current generation.
    pub fn generate_grid(
        &mut self,
    ) -> Result<(GenInfo, GridData<T, ModelInstance>), GenerationError> {
        let gen_info = self.internal_generate(&mut None)?;
        Ok((gen_info, self.internal_to_grid_data()))
    }

    pub fn generate_collected(&mut self) -> Result<(GenInfo, Vec<GridNode>), GenerationError> {
        let mut generated_nodes = Vec::new();
        let gen_info = self.internal_generate(&mut Some(&mut generated_nodes))?;
        Ok((gen_info, generated_nodes))
    }

    pub fn generate(&mut self) -> Result<GenInfo, GenerationError> {
        let gen_info = self.internal_generate(&mut None)?;
        Ok(gen_info)
    }

    fn internal_generate(&mut self, collector: &mut Collector) -> Result<GenInfo, GenerationError> {
        let mut last_error = None;
        for try_index in 0..=self.max_retry_count {
            #[cfg(feature = "debug-traces")]
            info!("Try n°{}", try_index + 1);

            if let Some(collector) = collector {
                collector.clear();
            }
            match self.generate_remaining_nodes(collector) {
                Ok(_) => {
                    return Ok(GenInfo {
                        try_count: try_index + 1,
                    })
                }
                Err(err) => {
                    last_error = Some(err);
                }
            }
        }
        Err(last_error.unwrap()) // We know that last_err is Some
    }

    /// Advances the generation by one "step": select a node and a model via the heuristics and propagate the changes.
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
        let mut generated_nodes = Vec::new();
        let res = self.internal_select_and_propagate(&mut Some(&mut generated_nodes))?;
        Ok((res, generated_nodes))
    }

    pub fn set_and_propagate_collected(
        &mut self,
        node_index: NodeIndex,
        model_variant_index: ModelVariantIndex,
    ) -> Result<(GenerationStatus, Vec<GridNode>), NodeSetError> {
        let mut generated_nodes = Vec::new();
        let res = self.internal_set_and_propagate(
            node_index,
            model_variant_index,
            &mut Some(&mut generated_nodes),
        )?;
        Ok((res, generated_nodes))
    }

    pub fn set_and_propagate(
        &mut self,
        node_index: NodeIndex,
        model_variant_index: ModelVariantIndex,
    ) -> Result<GenerationStatus, NodeSetError> {
        self.internal_set_and_propagate(node_index, model_variant_index, &mut None)
    }

    pub fn reinitialize(&mut self) -> GenerationStatus {
        self.internal_reinitialize(&mut None)
    }

    pub fn reinitialize_collected(&mut self) -> (GenerationStatus, Vec<GridNode>) {
        let mut generated_nodes = Vec::new();
        let res = self.internal_reinitialize(&mut Some(&mut generated_nodes));
        (res, generated_nodes)
    }

    fn pregen(&mut self, collector: &mut Collector) -> Result<GenerationStatus, NodeSetError> {
        self.initialize_supports_count(collector)?;
        // If done already, we still try to set all nodes and succeed only if initial nodes spawn requests match the already generated nodes.
        self.pregen_initial_nodes(collector)
    }

    fn pregen_initial_nodes(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, NodeSetError> {
        let initial_nodes = Arc::clone(&self.initial_nodes);
        for (node_index, model_variant_index) in initial_nodes.iter() {
            match self.check_set_and_propagate_parameters(*node_index, *model_variant_index)? {
                NodeSetStatus::AlreadySet => continue,
                NodeSetStatus::CanBeSet => (),
            }

            match self.unchecked_set_and_propagate(*node_index, *model_variant_index, collector)? {
                GenerationStatus::Ongoing => (),
                GenerationStatus::Done => return Ok(GenerationStatus::Done),
            }
        }
        // We can't be done here, unchecked_set_and_propagate would have seen it.
        Ok(GenerationStatus::Ongoing)
    }

    /// - reinitializes the generator if needed
    /// - returns `true` if the generation is [`GenerationStatus::Ongoing`] and the operation should continue, and false if the generation is [`GenerationStatus::Done`] and the operation should stop
    fn auto_reinitialize_and_continue(&mut self, collector: &mut Collector) -> bool {
        match self.status {
            InternalGeneratorStatus::Ongoing => true,
            InternalGeneratorStatus::Done | InternalGeneratorStatus::Failed(_) => {
                match self.internal_reinitialize(collector) {
                    GenerationStatus::Ongoing => true,
                    GenerationStatus::Done => false,
                }
            }
        }
    }

    /// Top-level handler of public API calls. Calls [`Generator::auto_reinitialize_and_continue`]
    fn internal_set_and_propagate(
        &mut self,
        node_index: NodeIndex,
        model_variant_index: ModelVariantIndex,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, NodeSetError> {
        if !self.auto_reinitialize_and_continue(collector) {
            return Ok(GenerationStatus::Done);
        };

        match self.check_set_and_propagate_parameters(node_index, model_variant_index)? {
            NodeSetStatus::AlreadySet => {
                // We can't be done here
                return Ok(GenerationStatus::Ongoing);
            }
            NodeSetStatus::CanBeSet => (),
        }

        Ok(self.unchecked_set_and_propagate(node_index, model_variant_index, collector)?)
    }

    /// Top-level handler of public API calls. Calls [`Generator::auto_reinitialize_and_continue`]
    fn internal_select_and_propagate(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GenerationError> {
        if !self.auto_reinitialize_and_continue(collector) {
            return Ok(GenerationStatus::Done);
        };

        self.unchecked_select_and_propagate(collector)
    }

    /// Top-level handler of public API calls. Calls [`Generator::auto_reinitialize_and_continue`]
    fn generate_remaining_nodes(
        &mut self,
        collector: &mut Collector,
    ) -> Result<(), GenerationError> {
        if !self.auto_reinitialize_and_continue(collector) {
            return Ok(());
        };

        // `nodes_left_to_generate` is an upper limit to the number of iterations. We avoid an unnecessary while loop.
        for _i in 0..self.nodes_left_to_generate {
            match self.unchecked_select_and_propagate(collector) {
                Ok(GenerationStatus::Done) => return Ok(()),
                Ok(GenerationStatus::Ongoing) => (),
                Err(e) => return Err(e),
            };
        }
        Ok(())
    }

    fn unchecked_select_and_propagate(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GenerationError> {
        let node_index = match self
            .node_selection_heuristic
            .select_node(&self.possible_models_counts, &mut self.rng)
        {
            Some(index) => index,
            None => {
                // TODO Here, should not be able to find None anymore.
                self.status = InternalGeneratorStatus::Done;
                return Ok(GenerationStatus::Done);
            }
        };
        // We found a node not yet generated. "Observe/collapse" the node: select a model for the node
        let selected_model_index = self.select_model(node_index);

        #[cfg(feature = "debug-traces")]
        debug!(
            "Heuristics selected model {:?} named '{}' for node {} at position {:?}",
            self.rules.model(selected_model_index),
            self.rules.name(selected_model_index),
            node_index,
            self.grid.get_position(node_index)
        );
        if !self.observers.is_empty() || collector.is_some() {
            self.signal_selection(collector, node_index, selected_model_index);
        }

        self.handle_selected(node_index, selected_model_index);

        if let Err(err) = self.propagate(collector) {
            self.signal_contradiction(err.node_index);
            return Err(err);
        };

        Ok(self.check_if_done())
    }

    /// Cannot fail since pre-gen was successful
    fn generate_initial_nodes(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GenerationError> {
        let initial_nodes = Arc::clone(&self.initial_nodes);
        for (node_index, model_variant_index) in initial_nodes.iter() {
            if self.possible_models_counts[*node_index] <= 1 {
                // This means node_index is already generated. And since pre-gen was successful, we know that it must be set to "model_variant_index" already. We skip this node.
                continue;
            }

            // This cannot fail
            match self.unchecked_set_and_propagate(*node_index, *model_variant_index, collector)? {
                GenerationStatus::Ongoing => (),
                GenerationStatus::Done => return Ok(GenerationStatus::Done),
            }
        }
        Ok(self.check_if_done())
    }

    /// - node_index and model_variant_index must be valid
    /// - model_variant_index must be possible on node_index
    /// - node_index must not be generated yet
    /// - Generator internal status must be [InternalGeneratorStatus::Ongoing]
    fn unchecked_set_and_propagate(
        &mut self,
        node_index: NodeIndex,
        model_variant_index: ModelVariantIndex,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GenerationError> {
        #[cfg(feature = "debug-traces")]
        debug!(
            "Set model {:?} named '{}' for node {} at position {:?}",
            self.rules.model(model_variant_index),
            self.rules.name(model_variant_index),
            node_index,
            self.grid.get_position(node_index)
        );

        if !self.observers.is_empty() {
            self.signal_selection(collector, node_index, model_variant_index);
        }

        self.handle_selected(node_index, model_variant_index);

        if let Err(err) = self.propagate(collector) {
            self.signal_contradiction(err.node_index);
            return Err(err);
        };

        Ok(self.check_if_done())
    }

    fn reset_with_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = StdRng::seed_from_u64(seed);

        self.status = InternalGeneratorStatus::Ongoing;

        let nodes_count = self.grid.total_size();
        self.nodes = bitvec![1;self.rules.models_count() * nodes_count ];
        self.nodes_left_to_generate = nodes_count;
        self.possible_models_counts = vec![self.rules.models_count(); nodes_count];
        self.propagation_stack = Vec::new();
        self.node_selection_heuristic.reinitialize();
    }

    /// Advances the seed
    fn internal_reinitialize(&mut self, collector: &mut Collector) -> GenerationStatus {
        // Gen next seed from current rng
        let next_seed = self.rng.gen::<u64>();
        self.reset_with_seed(next_seed);

        #[cfg(feature = "debug-traces")]
        info!(
            "Reinitializing generator with seed {}, state was {:?}",
            self.seed, self.status
        );

        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Reinitializing(self.seed));
        }

        // Since Pre-gen succeeded. The following calls will always succeed.
        let _ = self.initialize_supports_count(collector);
        self.generate_initial_nodes(collector).unwrap()
    }

    /// Returns an error if :
    /// - node_index is invalid
    /// - model_variant_index is invalid
    /// - model_variant_index is not possible on node_index
    /// Returns [`Ok(NodeSetStatus::CanBeSet)`] if model_variant_index can be generated on node_index and [`Ok(NodeSetStatus::AlreadySet)`] if node_index is already generated to model_variant_index
    fn check_set_and_propagate_parameters(
        &self,
        node_index: NodeIndex,
        model_variant_index: ModelVariantIndex,
    ) -> Result<NodeSetStatus, NodeSetError> {
        if model_variant_index > self.rules.models_count() {
            return Err(NodeSetError::InvalidModelIndex(model_variant_index));
        }
        if node_index > self.possible_models_counts.len() {
            return Err(NodeSetError::InvalidNodeIndex(node_index));
        }
        if !self.is_model_possible(node_index, model_variant_index) {
            return Err(NodeSetError::IllegalModel(model_variant_index, node_index));
        }
        if self.possible_models_counts[node_index] <= 1 {
            return Ok(NodeSetStatus::AlreadySet);
        }
        Ok(NodeSetStatus::CanBeSet)
    }

    /// Initialize the supports counts array. This may already start to generate/ban/... some nodes according to the given constraints.
    ///
    /// Returns `Ok` if the initialization went well and sets the internal status to [`InternalGeneratorStatus::Ongoing`] or [`InternalGeneratorStatus::Done`]. Else, sets the internal status to [`InternalGeneratorStatus::Failed`] and returns [`GenerationError`]
    fn initialize_supports_count(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GenerationError> {
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

        Ok(self.check_if_done())
    }

    fn check_if_done(&mut self) -> GenerationStatus {
        if self.nodes_left_to_generate == 0 {
            self.status = InternalGeneratorStatus::Done;
            GenerationStatus::Done
        } else {
            self.status = InternalGeneratorStatus::Ongoing;
            GenerationStatus::Ongoing
        }
    }

    fn handle_selected(&mut self, node_index: usize, selected_model_index: ModelVariantIndex) {
        // Iterate all the possible models because we don't have an easy way to iterate only the models possible at node_index. But we'll filter impossible models right away. TODO: benchmark iter_ones
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
                self.supports_count[(node_index, model_index, *dir as usize)] = 0;
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
    }

    fn signal_selection(
        &mut self,
        collector: &mut Collector,
        node_index: usize,
        model_index: ModelVariantIndex,
    ) {
        let grid_node = GridNode {
            node_index,
            model_instance: self.rules.model(model_index).clone(),
        };
        let update = GenerationUpdate::Generated(grid_node);
        for obs in &mut self.observers {
            let _ = obs.send(update);
        }
        if let Some(collector) = collector {
            collector.push(grid_node);
        }
        self.nodes_left_to_generate = self.nodes_left_to_generate.saturating_sub(1);
    }

    /// Returns [`GenerationError`] if a node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    fn propagate(&mut self, collector: &mut Collector) -> Result<(), GenerationError> {
        // Clone the ref to allow for mutability of other members in the interior loops
        let rules = Arc::clone(&self.rules);

        while let Some(from) = self.propagation_stack.pop() {
            let from_position = self.grid.get_position(from.node_index);

            #[cfg(feature = "debug-traces")]
            trace!(
                "Propagate removal of model {:?} named '{}' for node {}",
                self.rules.model(from.model_index),
                self.rules.name(from.model_index),
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

    fn enqueue_removal_to_propagate(&mut self, node_index: usize, model_index: ModelVariantIndex) {
        #[cfg(feature = "debug-traces")]
        trace!(
            "Enqueue removal for propagation: model {:?} named '{}' from node {}",
            self.rules.model(model_index),
            self.rules.name(model_index),
            node_index
        );
        self.propagation_stack.push(PropagationEntry {
            node_index,
            model_index,
        });
    }

    /// Returns [`GenerationError`] if the node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    ///
    /// Should only be called a model that is still possible for this node
    fn ban_model_from_node(
        &mut self,
        node_index: usize,
        model: usize,
        collector: &mut Collector,
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
            "Ban model {:?} named '{}' from node {} at position {:?}, {} models left",
            self.rules.model(model),
            self.rules.name(model),
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
                        "Previous bans force model {:?} named '{}' for node {} at position {:?}",
                        self.rules.model(forced_model),
                        self.rules.name(model),
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
                let possible_models: Vec<ModelVariantIndex> = (0..self.rules.models_count())
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
    fn internal_to_grid_data(&self) -> GridData<T, ModelInstance> {
        let mut generated_nodes = Vec::with_capacity(self.nodes.len());
        for node_index in 0..self.grid.total_size() {
            let model_index = self.get_model_index(node_index);
            generated_nodes.push(self.rules.model(model_index).clone())
        }

        GridData::new(self.grid.clone(), generated_nodes)
    }

    fn get_model_index(&self, node_index: usize) -> usize {
        self.nodes[node_index * self.rules.models_count()
            ..node_index * self.rules.models_count() + self.rules.models_count()]
            .first_one()
            .unwrap_or(0)
    }

    fn create_observer_queue(&mut self) -> crossbeam_channel::Receiver<GenerationUpdate> {
        // We can't simply bound to the number of nodes since we might retry some generations. (and send more than number_of_nodes updates)
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        receiver
    }

    fn signal_contradiction(&mut self, node_index: NodeIndex) {
        #[cfg(feature = "debug-traces")]
        debug!("Generation failed due to a contradiction");

        self.status = InternalGeneratorStatus::Failed(GenerationError { node_index });
        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Failed(node_index));
        }
    }
}
