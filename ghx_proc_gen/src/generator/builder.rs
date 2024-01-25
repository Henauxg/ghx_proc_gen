use std::{marker::PhantomData, sync::Arc};

use crate::{
    grid::{direction::CoordinateSystem, GridData, GridDefinition, NodeRef},
    InvalidGridSize, NodeSetError,
};

use super::{
    node_heuristic::NodeSelectionHeuristic,
    observer::{GenerationUpdate, QueuedObserver, QueuedStatefulObserver},
    rules::{ModelVariantRef, Rules},
    Collector, Generator, GridNode, ModelSelectionHeuristic, RngMode,
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
/// use ghx_proc_gen::{grid::GridDefinition, generator::{builder::GeneratorBuilder, rules::{Rules, RulesBuilder}, socket::{SocketsCartesian2D, SocketCollection}, model::ModelCollection}};
///
/// let mut sockets = SocketCollection::new();
/// let a = sockets.create();
/// sockets.add_connection(a, vec![a]);
///
/// let mut models = ModelCollection::new();
/// models.create(SocketsCartesian2D::Mono(a));
///
/// let rules = RulesBuilder::new_cartesian_2d(models,sockets).build().unwrap();
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
    initial_nodes_refs: Vec<(NodeRef, ModelVariantRef)>,
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
            initial_nodes_refs: Vec::new(),
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
            initial_nodes_refs: self.initial_nodes_refs,

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
            initial_nodes_refs: self.initial_nodes_refs,

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
            initial_nodes_refs: self.initial_nodes_refs,

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

    pub fn with_initial_nodes<N: Into<NodeRef>, M: Into<ModelVariantRef>>(
        mut self,
        initial_nodes: Vec<(N, M)>,
    ) -> Self {
        for (node_ref, model_ref) in initial_nodes {
            self.initial_nodes_refs
                .push((node_ref.into(), model_ref.into()));
        }
        self
    }
}

// For functions in this impl, we know that self.grid is `Some` thanks to the typing.
impl<C: CoordinateSystem, R> GeneratorBuilder<Set, R, C> {
    pub fn add_queued_stateful_observer(&mut self) -> QueuedStatefulObserver<C> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        let grid = self.grid.clone().unwrap();
        QueuedStatefulObserver::create(receiver, &grid)
    }

    pub fn add_queued_observer(&mut self) -> QueuedObserver {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        QueuedObserver::create(receiver)
    }

    pub fn with_initial_grid<M>(
        mut self,
        data: GridData<C, Option<M>>,
    ) -> Result<Self, InvalidGridSize>
    where
        for<'a> &'a M: Into<ModelVariantRef>,
    {
        let grid = self.grid.as_ref().unwrap();
        if grid.size() != data.grid().size() {
            return Err(InvalidGridSize(data.grid().size(), grid.size()));
        } else {
            for (node_index, node) in data.nodes().iter().enumerate() {
                match node {
                    Some(model_ref) => self
                        .initial_nodes_refs
                        .push((node_index.into(), model_ref.into())),
                    None => (),
                }
            }
            Ok(self)
        }
    }
}

impl<C: CoordinateSystem> GeneratorBuilder<Set, Set, C> {
    /// Instantiates a [`Generator`] as specified by the various builder parameters.
    pub fn build(self) -> Result<Generator<C>, NodeSetError> {
        self.internal_build(&mut None)
    }

    /// Instantiates a [`Generator`] as specified by the various builder parameters and return the initially generated nodes if any
    pub fn build_collected(self) -> Result<(Generator<C>, Vec<GridNode>), NodeSetError> {
        let mut generated_nodes = Vec::new();
        let res = self.internal_build(&mut Some(&mut generated_nodes))?;
        Ok((res, generated_nodes))
    }

    fn internal_build(self, collector: &mut Collector) -> Result<Generator<C>, NodeSetError> {
        // We know that self.rules and self.grid are `Some` thanks to the typing.
        let rules = self.rules.unwrap();
        let grid = self.grid.unwrap();

        // We don't fully check them here, simply dereference to obtain (NodeIndex, ModelVariantIndex) pairs.
        // Generator will fully verify them during pre-gen.
        let mut initial_nodes = Vec::with_capacity(self.initial_nodes_refs.len());
        for (node_ref, model_variant_ref) in self.initial_nodes_refs {
            let node_index = grid.index_from_ref(node_ref);
            let model_variant_index = rules.var_index_from_ref(model_variant_ref)?;
            initial_nodes.push((node_index, model_variant_index));
        }

        Generator::create(
            rules,
            grid,
            initial_nodes,
            self.max_retry_count,
            self.node_selection_heuristic,
            self.model_selection_heuristic,
            self.rng_mode,
            self.observers,
            collector,
        )
    }
}
