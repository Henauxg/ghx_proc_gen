use std::sync::Arc;

use bitvec::{bitvec, order::LocalBits, slice::IterOnes, vec::BitVec};
use ndarray::{Array, Ix3};
use rand::{
    distributions::{Distribution, WeightedIndex},
    rngs::StdRng,
    Rng, SeedableRng,
};

#[cfg(feature = "debug-traces")]
use tracing::{debug, info, trace};

use crate::{
    grid::{direction::CoordinateSystem, GridData, GridDefinition, NodeIndex},
    GeneratorError, NodeSetError,
};

use super::{
    model::{ModelInstance, ModelVariantIndex},
    node_heuristic::{InternalNodeSelectionHeuristic, NodeSelectionHeuristic},
    observer::GenerationUpdate,
    rules::Rules,
    Collector, GenInfo, GeneratedNode, GenerationStatus, ModelSelectionHeuristic, NodeSetStatus,
    RngMode,
};

#[derive(Default, Debug, Clone, Copy)]
pub(crate) enum InternalGeneratorStatus {
    /// Generation has not finished.
    #[default]
    Ongoing,
    /// Generation ended succesfully.
    Done,
    /// Generation failed due to a contradiction.
    Failed(GeneratorError),
}

struct PropagationEntry {
    node_index: NodeIndex,
    model_index: ModelVariantIndex,
}

pub(crate) struct InternalGenerator<C: CoordinateSystem> {
    // === Read-only configuration ===
    pub(crate) grid: GridDefinition<C>,
    pub(crate) rules: Arc<Rules<C>>,

    // === Generation state ===
    pub(crate) status: InternalGeneratorStatus,
    pub(crate) nodes_left_to_generate: usize,
    /// Observers signaled with updates of the nodes.
    pub(crate) observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,
    pub(crate) seed: u64,
    rng: StdRng,
    /// `nodes[node_index * self.rules.models_count() + model_index]` is true (1) if model with index `model_index` is still allowed on node with index `node_index`
    nodes: BitVec<usize>,
    /// Stores how many models are still possible for a given node
    possible_models_counts: Vec<usize>,
    node_selection_heuristic: InternalNodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,

    // === Constraint satisfaction algorithm data ===
    /// Stack of bans to propagate
    propagation_stack: Vec<PropagationEntry>,
    /// The value at `support_count[node_index][model_index][direction]` represents the number of supports of a `model_index` at `node_index` from `direction`
    supports_count: Array<usize, Ix3>,
}

impl<C: CoordinateSystem> InternalGenerator<C> {
    pub(crate) fn new(
        rules: Arc<Rules<C>>,
        grid: GridDefinition<C>,
        node_selection_heuristic: NodeSelectionHeuristic,
        model_selection_heuristic: ModelSelectionHeuristic,
        rng_mode: RngMode,
        observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,
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

        Self {
            grid,
            rules,

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
        }
    }
}

impl<C: CoordinateSystem> InternalGenerator<C> {
    #[inline]
    fn is_model_possible(&self, node: NodeIndex, model: ModelVariantIndex) -> bool {
        self.nodes[node * self.rules.models_count() + model] == true
    }

    #[inline]
    fn get_model_index(&self, node_index: NodeIndex) -> ModelVariantIndex {
        self.nodes[node_index * self.rules.models_count()
            ..node_index * self.rules.models_count() + self.rules.models_count()]
            .first_one()
            .unwrap_or(0)
    }

    #[inline]
    pub(crate) fn is_valid_node_index(&self, node_index: NodeIndex) -> bool {
        node_index < self.possible_models_counts.len()
    }

    pub(crate) fn possible_model_indexes(
        &self,
        node_index: NodeIndex,
    ) -> IterOnes<'_, ModelVariantIndex, LocalBits> {
        self.nodes[node_index * self.rules.models_count()
            ..node_index * self.rules.models_count() + self.rules.models_count()]
            .iter_ones()
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
    pub(crate) fn reinitialize(
        &mut self,
        collector: &mut Collector,
        initial_nodes: &Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> GenerationStatus {
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
        self.generate_initial_nodes(collector, initial_nodes)
            .unwrap()
    }

    /// Initialize the supports counts array. This may already start to generate/ban/... some nodes according to the given constraints.
    ///
    /// Returns `Ok` if the initialization went well and sets the internal status to [`InternalGeneratorStatus::Ongoing`] or [`InternalGeneratorStatus::Done`]. Else, sets the internal status to [`InternalGeneratorStatus::Failed`] and returns [`GeneratorError`]
    fn initialize_supports_count(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GeneratorError> {
        #[cfg(feature = "debug-traces")]
        debug!("Initializing support counts");

        let mut neighbours = vec![None; self.grid.directions().len()];
        for node in 0..self.grid.total_size() {
            // For a given `node`, `neighbours[direction]` will hold the optionnal index of the neighbour node in `direction`
            for direction in self.grid.directions() {
                let grid_pos = self.grid.pos_from_index(node);
                neighbours[*direction as usize] =
                    self.grid.get_next_index_in_direction(&grid_pos, *direction);
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

    /// Cannot fail since pre-gen was successful
    fn generate_initial_nodes(
        &mut self,
        collector: &mut Collector,
        initial_nodes: &Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> Result<GenerationStatus, GeneratorError> {
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

    pub(crate) fn pregen(
        &mut self,
        collector: &mut Collector,
        initial_nodes: &Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> Result<GenerationStatus, NodeSetError> {
        self.initialize_supports_count(collector)?;
        // If done already, we still try to set all nodes and succeed only if initial nodes spawn requests match the already generated nodes.
        self.pregen_initial_nodes(collector, initial_nodes)
    }

    fn pregen_initial_nodes(
        &mut self,
        collector: &mut Collector,
        initial_nodes: &Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> Result<GenerationStatus, NodeSetError> {
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
        if !self.is_valid_node_index(node_index) {
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

    pub(crate) fn generate(
        &mut self,
        collector: &mut Collector,
        retry_count: u32,
        initial_nodes: &Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> Result<GenInfo, GeneratorError> {
        let mut last_error = None;
        for try_index in 0..=retry_count {
            #[cfg(feature = "debug-traces")]
            info!("Try n°{}", try_index + 1);

            if let Some(collector) = collector {
                collector.clear();
            }
            match self.status {
                InternalGeneratorStatus::Ongoing => (),
                InternalGeneratorStatus::Done | InternalGeneratorStatus::Failed(_) => {
                    match self.reinitialize(collector, initial_nodes) {
                        GenerationStatus::Ongoing => (),
                        GenerationStatus::Done => {
                            return Ok(GenInfo {
                                try_count: try_index + 1,
                            })
                        }
                    }
                }
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

    /// Top-level handler of public API calls.
    fn generate_remaining_nodes(
        &mut self,
        collector: &mut Collector,
    ) -> Result<(), GeneratorError> {
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

    /// Top-level handler of public API calls.
    pub(crate) fn set_and_propagate(
        &mut self,
        node_index: NodeIndex,
        model_variant_index: ModelVariantIndex,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, NodeSetError> {
        match self.status {
            InternalGeneratorStatus::Ongoing => (),
            InternalGeneratorStatus::Done => return Ok(GenerationStatus::Done),
            InternalGeneratorStatus::Failed(err) => return Err(err.into()),
        }

        match self.check_set_and_propagate_parameters(node_index, model_variant_index)? {
            NodeSetStatus::AlreadySet => {
                // Nothing to do. We can't be done here
                return Ok(GenerationStatus::Ongoing);
            }
            NodeSetStatus::CanBeSet => (),
        }

        Ok(self.unchecked_set_and_propagate(node_index, model_variant_index, collector)?)
    }

    /// Top-level handler of public API calls.
    pub(crate) fn select_and_propagate(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GeneratorError> {
        match self.status {
            InternalGeneratorStatus::Ongoing => (),
            InternalGeneratorStatus::Done => return Ok(GenerationStatus::Done),
            InternalGeneratorStatus::Failed(err) => return Err(err),
        }

        self.unchecked_select_and_propagate(collector)
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
    ) -> Result<GenerationStatus, GeneratorError> {
        #[cfg(feature = "debug-traces")]
        debug!(
            "Set model {:?} named '{}' for node {} at position {:?}",
            self.rules.model(model_variant_index),
            self.rules.name_unchecked_str(model_variant_index),
            node_index,
            self.grid.pos_from_index(node_index)
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

    fn unchecked_select_and_propagate(
        &mut self,
        collector: &mut Collector,
    ) -> Result<GenerationStatus, GeneratorError> {
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
            self.rules.name_unchecked_str(selected_model_index),
            node_index,
            self.grid.pos_from_index(node_index)
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

    /// There should at least be one possible model for this node index. May panic otherwise.
    fn select_model(&mut self, node_index: NodeIndex) -> usize {
        match self.model_selection_heuristic {
            ModelSelectionHeuristic::WeightedProbability => {
                let possible_models: Vec<ModelVariantIndex> = (0..self.rules.models_count())
                    .filter(|&model_index| self.is_model_possible(node_index, model_index))
                    .collect();

                // TODO May cache the current sum of weights at each node.
                let weighted_distribution = WeightedIndex::new(
                    possible_models
                        .iter()
                        .map(|&model_index| self.rules.weight_unchecked(model_index)),
                )
                .unwrap();
                possible_models[weighted_distribution.sample(&mut self.rng)]
            }
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

    /// Returns [`GeneratorError`] if the node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    ///
    /// Should only be called a model that is still possible for this node
    fn ban_model_from_node(
        &mut self,
        node_index: usize,
        model: usize,
        collector: &mut Collector,
    ) -> Result<(), GeneratorError> {
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

        self.node_selection_heuristic.handle_ban(
            node_index,
            model,
            self.rules.weight_unchecked(model),
        );

        #[cfg(feature = "debug-traces")]
        trace!(
            "Ban model {:?} named '{}' from node {} at position {:?}, {} models left",
            self.rules.model(model),
            self.rules.name_unchecked_str(model),
            node_index,
            self.grid.pos_from_index(node_index),
            number_of_models_left
        );

        match *number_of_models_left {
            0 => return Err(GeneratorError { node_index }),
            1 => {
                #[cfg(feature = "debug-traces")]
                {
                    let forced_model = self.get_model_index(node_index);
                    debug!(
                        "Previous bans force model {:?} named '{}' for node {} at position {:?}",
                        self.rules.model(forced_model),
                        self.rules.name_unchecked_str(model),
                        node_index,
                        self.grid.pos_from_index(node_index)
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

    fn enqueue_removal_to_propagate(&mut self, node_index: usize, model_index: ModelVariantIndex) {
        #[cfg(feature = "debug-traces")]
        trace!(
            "Enqueue removal for propagation: model {:?} named '{}' from node {}",
            self.rules.model(model_index),
            self.rules.name_unchecked_str(model_index),
            node_index
        );
        self.propagation_stack.push(PropagationEntry {
            node_index,
            model_index,
        });
    }

    /// Returns [`GeneratorError`] if a node has no possible models left. Else, returns `Ok`.
    ///
    /// Does not modify the generator internal status.
    fn propagate(&mut self, collector: &mut Collector) -> Result<(), GeneratorError> {
        // Clone the ref to allow for mutability of other members in the interior loops
        let rules = Arc::clone(&self.rules);

        while let Some(from) = self.propagation_stack.pop() {
            let from_position = self.grid.pos_from_index(from.node_index);

            #[cfg(feature = "debug-traces")]
            trace!(
                "Propagate removal of model {:?} named '{}' for node {}",
                self.rules.model(from.model_index),
                self.rules.name_unchecked_str(from.model_index),
                from.node_index
            );

            // We want to update all the adjacent nodes (= in all directions)
            for dir in self.grid.directions() {
                // Get the adjacent node in this direction, it may not exist.
                if let Some(to_node_index) =
                    self.grid.get_next_index_in_direction(&from_position, *dir)
                {
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

    fn signal_selection(
        &mut self,
        collector: &mut Collector,
        node_index: NodeIndex,
        model_index: ModelVariantIndex,
    ) {
        let grid_node = GeneratedNode {
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

    fn signal_contradiction(&mut self, node_index: NodeIndex) {
        #[cfg(feature = "debug-traces")]
        debug!("Generation failed due to a contradiction");

        self.status = InternalGeneratorStatus::Failed(GeneratorError { node_index });
        for obs in &mut self.observers {
            let _ = obs.send(GenerationUpdate::Failed(node_index));
        }
    }

    /// Should only be called when the nodes are fully generated
    pub(crate) fn to_grid_data(&self) -> GridData<C, ModelInstance> {
        let mut generated_nodes = Vec::with_capacity(self.nodes.len());
        for node_index in 0..self.grid.total_size() {
            let model_index = self.get_model_index(node_index);
            generated_nodes.push(self.rules.model(model_index).clone())
        }

        GridData::new(self.grid.clone(), generated_nodes)
    }
}
