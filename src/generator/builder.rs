use std::{marker::PhantomData, rc::Rc};

use crate::grid::{GridCartesian2D, GridCartesian3D, GridTrait};
use bitvec::prelude::*;
use ndarray::Array;
use rand::thread_rng;

use super::{
    rules::{RulesCartesian2D, RulesCartesian3D, RulesTrait},
    Generator, ModelSelectionHeuristic, NodeSelectionHeuristic,
};

const DEFAULT_RETRY_COUNT: u32 = 10;

pub enum Set {}
pub enum Unset {}

pub struct GeneratorBuilder<G, R> {
    rules: Option<Rc<R>>,
    grid: Option<G>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    typestate: PhantomData<(G, R)>,
}

impl GeneratorBuilder<Unset, Unset> {
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

impl GeneratorBuilder<Unset, Unset> {
    pub fn with_rules<T: RulesTrait>(self, rules: T) -> GeneratorBuilder<Unset, T> {
        GeneratorBuilder {
            grid: self.grid,
            rules: Some(Rc::new(rules)),
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }

    pub fn with_shared_rules<T: RulesTrait>(self, rules: Rc<T>) -> GeneratorBuilder<Unset, T> {
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

impl GeneratorBuilder<Unset, RulesCartesian2D> {
    pub fn with_grid(
        self,
        grid: GridCartesian2D,
    ) -> GeneratorBuilder<GridCartesian2D, RulesCartesian2D> {
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

impl GeneratorBuilder<Unset, RulesCartesian3D> {
    pub fn with_grid(
        self,
        grid: GridCartesian3D,
    ) -> GeneratorBuilder<GridCartesian3D, RulesCartesian3D> {
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

impl<G, R> GeneratorBuilder<G, R> {
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

impl<G: GridTrait, R: RulesTrait> GeneratorBuilder<G, R> {
    pub fn build(self) -> Generator<G, R> {
        let rules = self.rules.unwrap();
        let models_count = rules.models_count();
        let grid = self.grid.unwrap();
        let direction_count = grid.directions().len();
        let nodes_count = grid.total_size();

        Generator {
            grid,
            rules,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,

            rng: thread_rng(),

            nodes: bitvec![1; nodes_count * models_count],
            possible_models_count: vec![models_count, nodes_count],

            propagation_stack: Vec::new(),
            supports_count: Array::zeros((nodes_count, models_count, direction_count)),
        }
    }
}
