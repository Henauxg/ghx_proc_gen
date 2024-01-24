use std::{marker::PhantomData, sync::Arc};

use crate::{
    grid::{direction::CoordinateSystem, GridDefinition, GridPosition, NodeIndex},
    NodeSetError,
};

use super::{
    model::{Model, ModelIndex, ModelRotation, ModelVariantIndex},
    node_heuristic::NodeSelectionHeuristic,
    observer::{GenerationUpdate, QueuedObserver, QueuedStatefulObserver},
    rules::Rules,
    Generator, GridNode, ModelSelectionHeuristic, RngMode,
};

/// Default retry count for the generator
pub const DEFAULT_RETRY_COUNT: u32 = 50;

/// Internal type used to provide a type-safe builder with compatible [`GridDefinition`] and [`Rules`]
pub enum Set {}
/// Internal type used to provide a type-safe builder with compatible [`GridDefinition`] and [`Rules`]
pub enum Unset {}

/// Used to instantiate a new [`Generator`].
///
/// [`Rules`] and [`GridDefinition`] are the two non-optionnal structs that are needed before being able to call `build`.
///
/// ### Example
///
/// Create a `Generator` from a `GeneratorBuilder`.
/// ```
/// use ghx_proc_gen::{grid::GridDefinition, generator::{builder::GeneratorBuilder, rules::{Rules, RulesBuilder}, socket::{SocketsCartesian2D, SocketCollection}}};
///
/// let mut sockets = SocketCollection::new();
/// let a = sockets.create();
/// sockets.add_connection(a, vec![a]);
///
/// let rules = RulesBuilder::new_cartesian_2d(
///     vec![SocketsCartesian2D::Mono(a).new_model()],
///     sockets).build().unwrap();
///
/// let grid = GridDefinition::new_cartesian_2d(10, 10, false, false);
/// let mut generator = GeneratorBuilder::new()
///    .with_rules(rules)
///    .with_grid(grid)
///    .build();
/// ```
pub struct GeneratorBuilder<G, R, C: CoordinateSystem> {
    rules: Option<Arc<Rules<C>>>,
    grid: Option<GridDefinition<C>>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    rng_mode: RngMode,
    observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,
    initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,
    initial_nodes_ref: Vec<(NodeRef, ModelVariantRef<C>)>,
    typestate: PhantomData<(G, R)>,
}

impl<C: CoordinateSystem> GeneratorBuilder<Unset, Unset, C> {
    /// Creates a [`GeneratorBuilder`] with its values set to their default.
    pub fn new() -> Self {
        Self {
            rules: None,
            grid: None,
            max_retry_count: DEFAULT_RETRY_COUNT,
            node_selection_heuristic: NodeSelectionHeuristic::MinimumRemainingValue,
            model_selection_heuristic: ModelSelectionHeuristic::WeightedProbability,
            rng_mode: RngMode::RandomSeed,
            observers: Vec::new(),
            initial_nodes: Vec::new(),
            initial_nodes_ref: Vec::new(),
            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem> GeneratorBuilder<Unset, Unset, C> {
    /// Sets the [`Rules`] to be used by the [`Generator`]
    pub fn with_rules(self, rules: Rules<C>) -> GeneratorBuilder<Unset, Set, C> {
        GeneratorBuilder {
            rules: Some(Arc::new(rules)),

            grid: self.grid,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            observers: self.observers,
            initial_nodes: self.initial_nodes,
            initial_nodes_ref: self.initial_nodes_ref,

            typestate: PhantomData,
        }
    }

    /// Sets the [`Rules`] to be used by the [`Generator`]. The `Generator` will hold a read-only Rc onto those `Rules` which can be safely shared by multiple `Generator`.
    pub fn with_shared_rules(self, rules: Arc<Rules<C>>) -> GeneratorBuilder<Unset, Set, C> {
        GeneratorBuilder {
            rules: Some(rules),

            grid: self.grid,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            observers: self.observers,
            initial_nodes: self.initial_nodes,
            initial_nodes_ref: self.initial_nodes_ref,

            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem> GeneratorBuilder<Unset, Set, C> {
    /// Sets the [`GridDefinition`] to be used by the [`Generator`].
    pub fn with_grid(self, grid: GridDefinition<C>) -> GeneratorBuilder<Set, Set, C> {
        GeneratorBuilder {
            grid: Some(grid),

            rules: self.rules,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            observers: self.observers,
            initial_nodes: self.initial_nodes,
            initial_nodes_ref: self.initial_nodes_ref,

            typestate: PhantomData,
        }
    }
}

impl<G, R, C: CoordinateSystem> GeneratorBuilder<G, R, C> {
    /// Specifies how many time the [`Generator`] should retry to generate the [`GridDefinition`] when a contradiction is encountered. Set to [`DEFAULT_RETRY_COUNT`] by default.
    pub fn with_max_retry_count(mut self, max_retry_count: u32) -> Self {
        self.max_retry_count = max_retry_count;
        self
    }
    /// Specifies the [`NodeSelectionHeuristic`] to be used by the [`Generator`]. Defaults to [`NodeSelectionHeuristic::MinimumRemainingValue`].
    pub fn with_node_heuristic(mut self, heuristic: NodeSelectionHeuristic) -> Self {
        self.node_selection_heuristic = heuristic;
        self
    }
    /// Specifies the [`ModelSelectionHeuristic`] to be used by the [`Generator`]. Defaults to [`ModelSelectionHeuristic::WeightedProbability`].
    pub fn with_model_heuristic(mut self, heuristic: ModelSelectionHeuristic) -> Self {
        self.model_selection_heuristic = heuristic;
        self
    }
    /// Specifies the [`RngMode`] to be used by the [`Generator`]. Defaults to [`RngMode::RandomSeed`].
    pub fn with_rng(mut self, rng_mode: RngMode) -> Self {
        self.rng_mode = rng_mode;
        self
    }
}

impl<T: CoordinateSystem> GeneratorBuilder<Set, Set, T> {
    pub fn add_queued_stateful_observer(&mut self) -> QueuedStatefulObserver<T> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        let grid = self.grid.clone().unwrap(); // We know that self.grid is `Some` thanks to the typing.
        QueuedStatefulObserver::create(receiver, &grid)
    }

    pub fn add_queued_observer(&mut self) -> QueuedObserver {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        QueuedObserver::create(receiver)
    }

    // TODO: Remove, covered by with_initial_nodes
    pub fn with_initial_nodes_raw(
        mut self,
        initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> Self {
        self.initial_nodes = initial_nodes;
        self
    }

    pub fn with_initial_nodes<N: Into<NodeRef>, M: Into<ModelVariantRef<T>>>(
        mut self,
        initial_nodes: Vec<(N, M)>,
    ) -> Self {
        for (node_ref, model_ref) in initial_nodes {
            self.initial_nodes_ref
                .push((node_ref.into(), model_ref.into()));
        }
        self
    }

    /// Instantiates a [`Generator`] as specified by the various builder parameters.
    pub fn build(self) -> Result<Generator<T>, NodeSetError> {
        // We know that self.rules and self.grid are `Some` thanks to the typing.
        let rules = self.rules.unwrap();
        let grid = self.grid.unwrap();
        Generator::create(
            rules,
            grid,
            self.initial_nodes,
            self.max_retry_count,
            self.node_selection_heuristic,
            self.model_selection_heuristic,
            self.rng_mode,
            self.observers,
            &mut None,
        )
    }

    pub fn build_collected(self) -> Result<(Generator<T>, Vec<GridNode>), NodeSetError> {
        // We know that self.rules and self.grid are `Some` thanks to the typing.
        let rules = self.rules.unwrap();
        let grid = self.grid.unwrap();
        let mut generated_nodes = Vec::new();
        let res = Generator::create(
            rules,
            grid,
            self.initial_nodes,
            self.max_retry_count,
            self.node_selection_heuristic,
            self.model_selection_heuristic,
            self.rng_mode,
            self.observers,
            &mut Some(&mut generated_nodes),
        )?;
        Ok((res, generated_nodes))
    }
}

pub enum NodeRef {
    Index(NodeIndex),
    Pos(GridPosition),
}

pub enum ModelVariantRef<C: CoordinateSystem> {
    VariantIndex(ModelVariantIndex),
    Model(Model<C>),                   // TODO use first rotation available
    ModelRot(Model<C>, ModelRotation), // TODO Same as above + Handle case if ModelRotation is not in Model.
    Index(ModelIndex, ModelRotation),
}

impl Into<NodeRef> for NodeIndex {
    fn into(self) -> NodeRef {
        NodeRef::Index(self)
    }
}
impl Into<NodeRef> for GridPosition {
    fn into(self) -> NodeRef {
        NodeRef::Pos(self)
    }
}

impl<C: CoordinateSystem> Into<ModelVariantRef<C>> for ModelVariantIndex {
    fn into(self) -> ModelVariantRef<C> {
        ModelVariantRef::VariantIndex(self)
    }
}
impl<C: CoordinateSystem> Into<ModelVariantRef<C>> for Model<C> {
    fn into(self) -> ModelVariantRef<C> {
        ModelVariantRef::Model(self)
    }
}
impl<C: CoordinateSystem> Into<ModelVariantRef<C>> for (ModelIndex, ModelRotation) {
    fn into(self) -> ModelVariantRef<C> {
        ModelVariantRef::Index(self.0, self.1)
    }
}
