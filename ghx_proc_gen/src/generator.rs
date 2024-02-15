use core::fmt;
use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "bevy")]
use bevy::ecs::component::Component;

use crate::{
    grid::{
        direction::{Cartesian2D, CoordinateSystem},
        GridData, GridDefinition, NodeIndex, NodeRef,
    },
    GeneratorError, NodeSetError,
};

use self::{
    builder::{GeneratorBuilder, Unset},
    internal_generator::{InternalGenerator, InternalGeneratorStatus},
    model::{ModelIndex, ModelInstance, ModelRotation, ModelVariantIndex},
    node_heuristic::NodeSelectionHeuristic,
    observer::GenerationUpdate,
    rules::{ModelInfo, ModelVariantRef, Rules},
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

pub(crate) mod internal_generator;

/// Defines a heuristic for the choice of a model among the possible ones when a node has been selected for generation.

#[derive(Default, Clone, Copy)]
pub enum ModelSelectionHeuristic {
    /// Choses a random model among the possible ones, weighted by each model weight.
    #[default]
    WeightedProbability,
}

/// Different ways to seed the RNG of the generator.
///
/// Note: No matter the selected mode, on each failed generation/reset, the generator will generate and use a new `u64` seed using the previous `u64` seed.
///
/// As an example: if a generation with 50 retries is requested with a seed `s1`, but the generations fails 14 times before finally succeeding with seed `s15`, requesting the generation with any of the seeds `s1`, `s2`, ... to `s15` will give the exact same final successful result. However, while `s1` will need to redo the 14 failed generations before succeeding,`s15` will directly generate the successfull result.
#[derive(Default, Clone, Copy)]
pub enum RngMode {
    /// The generator will use the given seed for its random source.
    ///
    Seeded(u64),
    /// The generator will use a random seed for its random source.
    ///
    /// The randomly generated seed can still be retrieved on the generator once created.
    #[default]
    RandomSeed,
}

/// Represents the current generation state, if not failed.
#[derive(Default, Clone, Copy, Eq, PartialEq, Debug)]
pub enum GenerationStatus {
    /// The generation has not ended yet.
    #[default]
    Ongoing,
    /// The generation ended succesfully. The whole grid is generated.
    Done,
}

/// Output of a [`Generator`] in the context of its [`crate::grid::GridDefinition`].
#[derive(Clone, Copy, Debug)]
pub struct GeneratedNode {
    /// Index of the node in the [`crate::grid::GridDefinition`]
    pub node_index: NodeIndex,
    /// Generated node data
    pub model_instance: ModelInstance,
}

/// Information about a generation*
#[derive(Clone, Copy, Debug)]
pub struct GenInfo {
    /// How many tries the generation took before succeeding
    pub try_count: u32,
}

enum NodeSetStatus {
    AlreadySet,
    CanBeSet,
}

type Collector<'a> = Option<&'a mut Vec<GeneratedNode>>;

/// Model synthesis/WFC generator.
/// Use a [`GeneratorBuilder`] to get an instance of a [`Generator`].
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Generator<C: CoordinateSystem> {
    // === Dynamic configuration ===
    max_retry_count: u32,
    initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,

    // === Internal state ===
    internal: InternalGenerator<C>,
}

impl<C: CoordinateSystem> Generator<C> {
    /// Returns a new `GeneratorBuilder`
    pub fn builder() -> GeneratorBuilder<Unset, Unset, Cartesian2D> {
        GeneratorBuilder::new()
    }

    fn create(
        rules: Arc<Rules<C>>,
        grid: GridDefinition<C>,
        initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,
        max_retry_count: u32,
        node_selection_heuristic: NodeSelectionHeuristic,
        model_selection_heuristic: ModelSelectionHeuristic,
        rng_mode: RngMode,
        observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,
        collector: &mut Collector,
    ) -> Result<Self, NodeSetError> {
        let mut generator = Self {
            max_retry_count,
            initial_nodes,
            internal: InternalGenerator::new(
                rules,
                grid,
                node_selection_heuristic,
                model_selection_heuristic,
                rng_mode,
                observers,
            ),
        };
        match generator
            .internal
            .pregen(collector, &generator.initial_nodes)
        {
            Ok(_status) => Ok(generator),
            Err(err) => Err(err),
        }
    }

    /// Returns the `max_retry_count`: how many time the [`Generator`] should retry to generate the [`GridDefinition`] when a contradiction is encountered
    pub fn max_retry_count(&self) -> u32 {
        self.max_retry_count
    }

    /// Specifies how many time the [`Generator`] should retry to generate the [`GridDefinition`] when a contradiction is encountered
    pub fn set_max_retry_count(&mut self, max_retry_count: u32) {
        self.max_retry_count = max_retry_count;
    }

    /// Returns the seed that was used to initialize the generator RNG for this generation. See [`RngMode`] for more information.
    pub fn seed(&self) -> u64 {
        self.internal.seed
    }

    /// Returns the [`GridDefinition`] used by the generator
    pub fn grid(&self) -> &GridDefinition<C> {
        &self.internal.grid
    }

    /// Returns the [`Rules`] used by the generator
    pub fn rules(&self) -> &Rules<C> {
        &self.internal.rules
    }

    /// Returns how many nodes are left to generate
    pub fn nodes_left(&self) -> usize {
        self.internal.nodes_left_to_generate
    }

    /// Returns a [`GridData`] of [`ModelInstance`] with all the nodes generated if the generation is done
    ///
    /// Returns `None` if the generation is still ongoing or currently failed
    pub fn to_grid_data(&self) -> Option<GridData<C, ModelInstance>> {
        match self.internal.status {
            InternalGeneratorStatus::Ongoing => None,
            InternalGeneratorStatus::Failed(_) => None,
            InternalGeneratorStatus::Done => Some(self.internal.to_grid_data()),
        }
    }

    /// Tries to generate the whole grid. If the generation fails due to a contradiction, it will retry `max_retry_count` times before returning the last encountered [`GeneratorError`]
    ///
    /// If the generation is currently done or failed, calling this method will reinitialize the generator with the next seed before starting the generation.
    ///
    /// If the generation was already started by previous calls to [`Generator::set_and_propagate`] or [`Generator::select_and_propagate`], this will simply continue the current generation.
    pub fn generate_grid(
        &mut self,
    ) -> Result<(GenInfo, GridData<C, ModelInstance>), GeneratorError> {
        let gen_info =
            self.internal
                .generate(&mut None, self.max_retry_count, &self.initial_nodes)?;
        Ok((gen_info, self.internal.to_grid_data()))
    }

    /// Same as [`Generator::generate_grid`] but does not return the generated [`ModelInstance`] when successful.
    ///
    /// [`Generator::to_grid_data`] can still be called to retrieve a [`GridData`] afterwards.
    pub fn generate(&mut self) -> Result<GenInfo, GeneratorError> {
        let gen_info =
            self.internal
                .generate(&mut None, self.max_retry_count, &self.initial_nodes)?;
        Ok(gen_info)
    }

    /// Advances the generation by one "step": select a node and a model via the heuristics and propagate the changes.
    /// - Returns the [`GenerationStatus`] if the step executed successfully
    /// - Returns a [`GeneratorError`] if the generation fails due to a contradiction.
    ///
    /// If the generation is currently done or failed, this method will just return the done or failed status/error.
    ///
    /// **Note**: One call to this method **can** lead to more than one node generated if the propagation phase forces some other node(s) into a definite state (due to only one possible model remaining on a node)
    pub fn select_and_propagate(&mut self) -> Result<GenerationStatus, GeneratorError> {
        self.internal.select_and_propagate(&mut None)
    }

    /// Same as [`Generator::select_and_propagate`] but collects and return the generated [`GeneratedNode`] when successful.
    pub fn select_and_propagate_collected(
        &mut self,
    ) -> Result<(GenerationStatus, Vec<GeneratedNode>), GeneratorError> {
        let mut generated_nodes = Vec::new();
        let status = self
            .internal
            .select_and_propagate(&mut Some(&mut generated_nodes))?;
        Ok((status, generated_nodes))
    }

    /// Tries to set the node referenced by `node_ref` to the model refrenced by `model_variant_ref`. Then tries to propagate the change.
    /// - Returns `Ok` and the current [`GenerationStatus`] if successful.
    /// - Returns a [`NodeSetError`] if it fails.
    ///
    /// If the generation is currently done or failed, this method will just return the done or failed status/error.
    ///
    /// **Note**: One call to this method **can** lead to more than one node generated if the propagation phase forces some other node(s) into a definite state (due to only one possible model remaining on a node)
    pub fn set_and_propagate<N: NodeRef<C>, M: ModelVariantRef<C>>(
        &mut self,
        node_ref: N,
        model_variant_ref: M,
        memorized: bool,
    ) -> Result<GenerationStatus, NodeSetError> {
        let node_index = node_ref.to_index(&self.internal.grid);
        let model_variant_index = model_variant_ref.to_index(&self.internal.rules)?;
        let status = self
            .internal
            .set_and_propagate(node_index, model_variant_index, &mut None)?;
        if memorized {
            self.initial_nodes.push((node_index, model_variant_index));
        }
        Ok(status)
    }

    /// Same as [`Generator::set_and_propagate`] but also returns all the [`GeneratedNode`] generated by this generation operation if successful.
    pub fn set_and_propagate_collected<N: NodeRef<C>, M: ModelVariantRef<C>>(
        &mut self,
        node_ref: N,
        model_variant_ref: M,
        memorized: bool,
    ) -> Result<(GenerationStatus, Vec<GeneratedNode>), NodeSetError> {
        let mut generated_nodes = Vec::new();
        let node_index = node_ref.to_index(&self.internal.grid);
        let model_variant_index = model_variant_ref.to_index(&self.internal.rules)?;
        let status = self.internal.set_and_propagate(
            node_index,
            model_variant_index,
            &mut Some(&mut generated_nodes),
        )?;
        if memorized {
            self.initial_nodes.push((node_index, model_variant_index));
        }
        Ok((status, generated_nodes))
    }

    /// Reinitalizes the generator with the next seed (a seed is generated from the current seed)
    pub fn reinitialize(&mut self) -> GenerationStatus {
        self.internal.reinitialize(&mut None, &self.initial_nodes)
    }

    /// Same as [`Generator::reinitialize`] but also returns all the [`GeneratedNode`] generated by this generation operation.
    pub fn reinitialize_collected(&mut self) -> (GenerationStatus, Vec<GeneratedNode>) {
        let mut generated_nodes = Vec::new();
        let res = self
            .internal
            .reinitialize(&mut Some(&mut generated_nodes), &self.initial_nodes);
        (res, generated_nodes)
    }

    pub fn get_models_on(&self, node_index: NodeIndex) -> Vec<ModelInstance> {
        let mut models = Vec::new();
        if !self.internal.is_valid_node_index(node_index) {
            return models;
        }
        for model_variant_index in self.internal.possible_model_indexes(node_index) {
            models.push(self.internal.rules.model(model_variant_index).clone());
        }

        models
    }

    pub fn get_models_variations_on(&self, node_index: NodeIndex) -> (Vec<ModelVariations>, u32) {
        let mut model_variations = Vec::new();
        let mut total_models_count = 0;

        if !self.internal.is_valid_node_index(node_index) {
            return (model_variations, total_models_count);
        }

        let mut id_mapping = HashMap::new();
        for model_variant_index in self.internal.possible_model_indexes(node_index) {
            total_models_count += 1;
            let model = self.internal.rules.model(model_variant_index);
            let group_id = id_mapping
                .entry(model.model_index)
                .or_insert(model_variations.len());
            if *group_id == model_variations.len() {
                model_variations.push(ModelVariations {
                    index: model.model_index,
                    info: self.internal.rules.model_info(model_variant_index),
                    rotations: vec![model.rotation],
                });
            } else {
                model_variations[*group_id].rotations.push(model.rotation);
            }
        }

        (model_variations, total_models_count)
    }

    fn create_observer_queue(&mut self) -> crossbeam_channel::Receiver<GenerationUpdate> {
        // We can't simply bound to the number of nodes since we might retry some generations. (and send more than number_of_nodes updates)
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.internal.observers.push(sender);
        receiver
    }
}

#[derive(Debug, Clone)]
pub struct ModelVariations {
    pub index: ModelIndex,
    pub info: ModelInfo,
    pub rotations: Vec<ModelRotation>,
}

impl fmt::Display for ModelVariations {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.rotations.len() == 1 {
            write!(
                f,
                "id: {}, {}, rotation: {:?}",
                self.index, self.info, self.rotations[0]
            )
        } else {
            write!(
                f,
                "id: {}, {}, rotations: {:?}",
                self.index, self.info, self.rotations
            )
        }
    }
}
