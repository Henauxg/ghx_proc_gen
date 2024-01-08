use std::{marker::PhantomData, sync::Arc};

use crate::grid::{direction::DirectionSet, GridDefinition};

use super::{
    node_heuristic::NodeSelectionHeuristic, rules::Rules, Generator, ModelSelectionHeuristic,
    RngMode,
};

pub const DEFAULT_RETRY_COUNT: u32 = 10;

/// Internal type used to provide a type-safe builder with compatible [`GridDefinition`] and [`Rules`]
pub enum Set {}
/// Internal type used to provide a type-safe builder with compatible [`GridDefinition`] and [`Rules`]
pub enum Unset {}

/// Used to instantiate a new [`Generator`]. [`Rules`] and [`GridDefinition`] are the two non-optionnal structs that are needed before being able to call `build`.
/// ### Example
///
/// Create a `Generator` from a `GeneratorBuilder`.
/// ```
/// use ghx_proc_gen::{grid::GridDefinition, generator::{builder::GeneratorBuilder, rules::Rules, node::SocketsCartesian2D}};
///
/// let grid = GridDefinition::new_cartesian_2d(10, 10, false);
/// let rules = Rules::new_cartesian_2d(
///     vec![SocketsCartesian2D::Mono(0).new_model()],
///     vec![(0, vec![0])]).unwrap();
/// let mut generator = GeneratorBuilder::new()
///    .with_rules(rules)
///    .with_grid(grid)
///    .build();
/// ```
pub struct GeneratorBuilder<G, R, T: DirectionSet + Clone> {
    rules: Option<Arc<Rules<T>>>,
    grid: Option<GridDefinition<T>>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    rng_mode: RngMode,
    typestate: PhantomData<(G, R)>,
}

impl<T: DirectionSet + Clone> GeneratorBuilder<Unset, Unset, T> {
    /// Creates a [`GeneratorBuilder`] with its values set to their default.
    pub fn new() -> Self {
        Self {
            rules: None,
            grid: None,
            max_retry_count: DEFAULT_RETRY_COUNT,
            node_selection_heuristic: NodeSelectionHeuristic::MinimumRemainingValue,
            model_selection_heuristic: ModelSelectionHeuristic::WeightedProbability,
            rng_mode: RngMode::RandomSeed,
            typestate: PhantomData,
        }
    }
}

impl<T: DirectionSet + Clone> GeneratorBuilder<Unset, Unset, T> {
    /// Sets the [`Rules`] to be used by the [`Generator`]
    pub fn with_rules(self, rules: Rules<T>) -> GeneratorBuilder<Unset, Set, T> {
        GeneratorBuilder {
            grid: self.grid,
            rules: Some(Arc::new(rules)),
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            typestate: PhantomData,
        }
    }

    /// Sets the [`Rules`] to be used by the [`Generator`]. The `Generator` will hold a read-only Rc onto those `Rules` which can be safely shared by multiple `Generator`.
    pub fn with_shared_rules(self, rules: Arc<Rules<T>>) -> GeneratorBuilder<Unset, Set, T> {
        GeneratorBuilder {
            grid: self.grid,
            rules: Some(rules),
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            typestate: PhantomData,
        }
    }
}

impl<T: DirectionSet + Clone> GeneratorBuilder<Unset, Set, T> {
    /// Sets the [`GridDefinition`] to be used by the [`Generator`].
    pub fn with_grid(self, grid: GridDefinition<T>) -> GeneratorBuilder<Set, Set, T> {
        GeneratorBuilder {
            grid: Some(grid),
            rules: self.rules,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng_mode: self.rng_mode,
            typestate: PhantomData,
        }
    }
}

impl<G, R, T: DirectionSet + Clone> GeneratorBuilder<G, R, T> {
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

impl<T: DirectionSet + Clone> GeneratorBuilder<Set, Set, T> {
    /// Instantiates a [`Generator`] as specified by the various builder parameters.
    pub fn build(self) -> Generator<T> {
        // We know that self.rules and self.grid are `Some` thanks to the typing.
        let rules = self.rules.unwrap();
        let grid = self.grid.unwrap();
        Generator::new(
            rules,
            grid,
            self.max_retry_count,
            self.node_selection_heuristic,
            self.model_selection_heuristic,
            self.rng_mode,
        )
    }
}
