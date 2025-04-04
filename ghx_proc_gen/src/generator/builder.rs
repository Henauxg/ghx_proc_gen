use std::{marker::PhantomData, sync::Arc};

use ghx_grid::{
    coordinate_system::CoordinateSystem,
    grid::{Grid, GridData, NodeRef},
};

use crate::{GeneratorBuilderError, NodeIndex};

use super::{
    model::ModelVariantIndex,
    node_heuristic::NodeSelectionHeuristic,
    observer::{GenerationUpdate, QueuedObserver, QueuedStatefulObserver},
    rules::{ModelVariantRef, Rules},
    Collector, GeneratedNode, Generator, ModelSelectionHeuristic, RngMode,
};

/// Default retry count for the generator
pub const DEFAULT_RETRY_COUNT: u32 = 50;

/// Internal type used to provide a type-safe builder with compatible [`Grid`] and [`Rules`]
#[derive(Copy, Clone)]
pub struct Set;
/// Internal type used to provide a type-safe builder with compatible [`Grid`] and [`Rules`]
#[derive(Copy, Clone)]
pub struct Unset;

/// Used to instantiate a new [`Generator`].
///
/// [`Rules`] and [`Grid`] are the two non-optionnal structs that are needed before being able to call `build`.
///
/// ### Example
///
/// Create a `Generator` from a `GeneratorBuilder`.
/// ```
/// use ghx_proc_gen::{generator::{builder::GeneratorBuilder, rules::{Rules, RulesBuilder}, socket::{SocketsCartesian2D, SocketCollection}, model::ModelCollection}};
/// use ghx_grid::cartesian::grid::CartesianGrid;
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
/// let grid = CartesianGrid::new_cartesian_2d(10, 10, false, false);
/// let mut generator = GeneratorBuilder::new()
///    .with_rules(rules)
///    .with_grid(grid)
///    .build();
/// ```
#[derive(Clone)]
pub struct GeneratorBuilder<G, R, C: CoordinateSystem, T: Grid<C>> {
    rules: Option<Arc<Rules<C>>>,
    grid: Option<T>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    rng_mode: RngMode,
    observers: Vec<crossbeam_channel::Sender<GenerationUpdate>>,
    initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,
    typestate: PhantomData<(G, R)>,
}

impl<C: CoordinateSystem, G: Grid<C>> GeneratorBuilder<Unset, Unset, C, G> {
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
            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem, G: Grid<C>> GeneratorBuilder<Unset, Unset, C, G> {
    /// Sets the [`Rules`] to be used by the [`Generator`]
    pub fn with_rules(self, rules: Rules<C>) -> GeneratorBuilder<Unset, Set, C, G> {
        GeneratorBuilder {
            rules: Some(Arc::new(rules)),

            grid: self.grid,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            observers: self.observers,
            initial_nodes: self.initial_nodes,

            typestate: PhantomData,
        }
    }

    /// Sets the [`Rules`] to be used by the [`Generator`]. The `Generator` will hold a read-only Rc onto those `Rules` which can be safely shared by multiple `Generator`.
    pub fn with_shared_rules(self, rules: Arc<Rules<C>>) -> GeneratorBuilder<Unset, Set, C, G> {
        GeneratorBuilder {
            rules: Some(rules),

            grid: self.grid,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            observers: self.observers,
            initial_nodes: self.initial_nodes,

            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem, G: Grid<C>> GeneratorBuilder<Unset, Set, C, G> {
    /// Sets the [`Grid`] to be used by the [`Generator`].
    pub fn with_grid(self, grid: G) -> GeneratorBuilder<Set, Set, C, G> {
        GeneratorBuilder {
            grid: Some(grid),

            rules: self.rules,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            observers: self.observers,
            initial_nodes: self.initial_nodes,

            typestate: PhantomData,
        }
    }
}

impl<G, R, C: CoordinateSystem, T: Grid<C>> GeneratorBuilder<G, R, C, T> {
    /// Specifies how many time the [`Generator`] should retry to generate the [`Grid`] when a contradiction is encountered. Set to [`DEFAULT_RETRY_COUNT`] by default.
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

    /// Registers some [`NodeIndex`] [`ModelVariantIndex`] pairs to be spawned initially by the [`Generator`]. These nodes will be spawned when the generator reinitializes too.
    ///
    /// See [`GeneratorBuilder::with_initial_nodes`] for a more versatile and easy to use method (at the price of a bit of performances during the method call).
    pub fn with_initial_nodes_raw(
        mut self,
        initial_nodes: Vec<(NodeIndex, ModelVariantIndex)>,
    ) -> Self {
        self.initial_nodes.extend(initial_nodes);
        self
    }
}

// For functions in this impl, we know that self.grid is `Some` thanks to the typing.
impl<C: CoordinateSystem, R, G: Grid<C>> GeneratorBuilder<Set, R, C, G> {
    /// Adds a [`QueuedStatefulObserver`] to the [`Generator`] that will be built, and returns it.
    ///
    /// Adding the observer before building the generator allows the observer to see the nodes than *can* be generated during a generator's initialization.
    pub fn add_queued_stateful_observer(&mut self) -> QueuedStatefulObserver<C, G> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        let grid = self.grid.clone().unwrap();
        QueuedStatefulObserver::create(receiver, &grid)
    }

    /// Adds a [`QueuedObserver`] to the [`Generator`] that will be built, and returns it.
    ///
    /// Adding the observer before building the generator allows the observer to see the nodes than *can* be generated during a generator's initialization.
    pub fn add_queued_observer(&mut self) -> QueuedObserver {
        let (sender, receiver) = crossbeam_channel::unbounded();
        self.observers.push(sender);
        QueuedObserver::create(receiver)
    }

    /// Registers [`ModelVariantRef`] from a [`GridData`] to be spawned initially by the [`Generator`]. These nodes will be spawned when the generator reinitializes too.
    ///
    /// See [`GeneratorBuilder::with_initial_grid`] for a more versatile and easy to use method (at the price of a bit of performances during the method call).
    pub fn with_initial_grid_raw<M: ModelVariantRef<C>>(
        mut self,
        data: GridData<C, Option<ModelVariantIndex>, G>,
    ) -> Result<Self, GeneratorBuilderError> {
        let grid = self.grid.as_ref().unwrap();
        if grid.total_size() != data.grid().total_size() {
            return Err(GeneratorBuilderError::InvalidGridSize(
                data.grid().total_size(),
                grid.total_size(),
            ));
        } else {
            for (node_index, node) in data.iter().enumerate() {
                match node {
                    Some(model_var_index) => {
                        self.initial_nodes.push((node_index, *model_var_index))
                    }
                    None => (),
                }
            }
            Ok(self)
        }
    }
}

// For functions in this impl, we know that self.rules and self.grid are `Some` thanks to the typing.
impl<C: CoordinateSystem, G: Grid<C>> GeneratorBuilder<Set, Set, C, G> {
    /// Registers some [`NodeRef`] [`ModelVariantRef`] pairs to be spawned initially by the [`Generator`]. These nodes will be spawned when the generator reinitializes too.
    ///
    /// See [`GeneratorBuilder::with_initial_nodes_raw`] for a bit more performant but more constrained method. The performance difference only matters during this method call in the `GeneratorBuilder`, during generation all the initial nodes are already converted to their raw format.
    pub fn with_initial_nodes<N: NodeRef<C, G>, M: ModelVariantRef<C>>(
        mut self,
        initial_nodes: Vec<(N, M)>,
    ) -> Result<Self, GeneratorBuilderError> {
        let grid = self.grid.as_ref().unwrap();
        let rules = self.rules.as_ref().unwrap();
        for (node_ref, model_ref) in initial_nodes {
            self.initial_nodes
                .push((node_ref.to_index(grid), model_ref.to_index(rules)?));
        }
        Ok(self)
    }

    /// Registers [`ModelVariantRef`] from a [`GridData`] to be spawned initially by the [`Generator`]. These nodes will be spawned when the generator reinitializes too.
    ///
    /// See [`GeneratorBuilder::with_initial_grid_raw`] for a bit more performant but more constrained method. The performance difference only matters during this method call in the `GeneratorBuilder`, during generation all the initial nodes are already converted to their raw format.
    pub fn with_initial_grid<M: ModelVariantRef<C>>(
        mut self,
        data: GridData<C, Option<M>, G>,
    ) -> Result<Self, GeneratorBuilderError> {
        let grid = self.grid.as_ref().unwrap();
        let rules = self.rules.as_ref().unwrap();
        if grid.total_size() != data.grid().total_size() {
            return Err(GeneratorBuilderError::InvalidGridSize(
                data.grid().total_size(),
                grid.total_size(),
            ));
        } else {
            for (node_index, node) in data.iter().enumerate() {
                match node {
                    Some(model_ref) => self
                        .initial_nodes
                        .push((node_index, model_ref.to_index(rules)?)),
                    None => (),
                }
            }
            Ok(self)
        }
    }

    /// Instantiates a [`Generator`] as specified by the various builder parameters.
    pub fn build(self) -> Result<Generator<C, G>, GeneratorBuilderError> {
        self.internal_build(&mut None)
    }

    /// Instantiates a [`Generator`] as specified by the various builder parameters and return the initially generated nodes if any
    pub fn build_collected(
        self,
    ) -> Result<(Generator<C, G>, Vec<GeneratedNode>), GeneratorBuilderError> {
        let mut generated_nodes = Vec::new();
        let res = self.internal_build(&mut Some(&mut generated_nodes))?;
        Ok((res, generated_nodes))
    }

    fn internal_build(
        self,
        collector: &mut Collector,
    ) -> Result<Generator<C, G>, GeneratorBuilderError> {
        // We know that self.rules and self.grid are `Some` thanks to the typing.
        let rules = self.rules.unwrap();
        let grid = self.grid.unwrap();
        Ok(Generator::create(
            rules,
            grid,
            self.initial_nodes,
            self.max_retry_count,
            self.node_selection_heuristic,
            self.model_selection_heuristic,
            self.rng_mode,
            self.observers,
            collector,
        )?)
    }
}
