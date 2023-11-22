use std::{marker::PhantomData, rc::Rc};

use crate::grid::{
    direction::{Cartesian2D, Cartesian3D, DirectionSet},
    Grid,
};

use super::{rules::Rules, Generator, ModelSelectionHeuristic, NodeSelectionHeuristic};

const DEFAULT_RETRY_COUNT: u32 = 10;

pub enum Set {}
pub enum Unset {}

pub struct GeneratorBuilder<G, R, T: DirectionSet> {
    rules: Option<Rc<Rules<T>>>,
    grid: Option<Grid<T>>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    typestate: PhantomData<(G, R)>,
}

impl GeneratorBuilder<Unset, Unset, Cartesian2D> {
    pub fn new() -> Self {
        Self {
            rules: None,
            grid: None,
            max_retry_count: DEFAULT_RETRY_COUNT,
            node_selection_heuristic: NodeSelectionHeuristic::MinimumRemainingValue,
            model_selection_heuristic: ModelSelectionHeuristic::WeightedProbability,
            typestate: PhantomData,
        }
    }
}

impl<T: DirectionSet> GeneratorBuilder<Unset, Unset, T> {
    pub fn with_rules(self, rules: Rules<T>) -> GeneratorBuilder<Unset, Set, T> {
        GeneratorBuilder {
            grid: self.grid,
            rules: Some(Rc::new(rules)),
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }

    pub fn with_shared_rules(self, rules: Rc<Rules<T>>) -> GeneratorBuilder<Unset, Set, T> {
        GeneratorBuilder {
            grid: self.grid,
            rules: Some(rules),
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }
}

impl GeneratorBuilder<Unset, Set, Cartesian2D> {
    pub fn with_grid(self, grid: Grid<Cartesian2D>) -> GeneratorBuilder<Set, Set, Cartesian2D> {
        GeneratorBuilder {
            grid: Some(grid),
            rules: self.rules,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }
}

impl GeneratorBuilder<Unset, Set, Cartesian3D> {
    pub fn with_grid(self, grid: Grid<Cartesian3D>) -> GeneratorBuilder<Set, Set, Cartesian3D> {
        GeneratorBuilder {
            grid: Some(grid),
            rules: self.rules,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }
}

impl<G, R, T: DirectionSet> GeneratorBuilder<G, R, T> {
    pub fn with_max_retry_count(mut self, max_retry_count: u32) -> Self {
        self.max_retry_count = max_retry_count;
        self
    }

    pub fn with_node_heuristic(mut self, heuristic: NodeSelectionHeuristic) -> Self {
        self.node_selection_heuristic = heuristic;
        self
    }

    pub fn with_model_heuristic(mut self, heuristic: ModelSelectionHeuristic) -> Self {
        self.model_selection_heuristic = heuristic;
        self
    }
}

impl<T: DirectionSet> GeneratorBuilder<Set, Set, T> {
    pub fn build(self) -> Generator<T> {
        let rules = self.rules.unwrap();
        let grid = self.grid.unwrap();
        Generator::new(
            rules,
            grid,
            self.max_retry_count,
            self.node_selection_heuristic,
            self.model_selection_heuristic,
        )
    }
}
