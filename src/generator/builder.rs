use std::{marker::PhantomData, rc::Rc};

use bitvec::prelude::*;
use ndarray::Array;
use rand::thread_rng;

use crate::grid::Grid;

use super::{
    node::{expand_models, ExpandedNodeModel, NodeModel},
    Generator, ModelSelectionHeuristic, NodeSelectionHeuristic,
};

const DEFAULT_RETRY_COUNT: u32 = 10;

pub enum Set {}
pub enum Unset {}

pub struct GeneratorBuilder<G, M> {
    models: Option<Vec<ExpandedNodeModel>>,
    grid: Option<Grid>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,
    typestate: PhantomData<(G, M)>,
}

impl GeneratorBuilder<Unset, Unset> {
    pub fn new() -> Self {
        Self {
            models: None,
            grid: None,
            max_retry_count: DEFAULT_RETRY_COUNT,
            node_selection_heuristic: NodeSelectionHeuristic::MinimumRemainingValue,
            model_selection_heuristic: ModelSelectionHeuristic::WeightedProbability,
            typestate: PhantomData,
        }
    }
}

impl<M> GeneratorBuilder<Unset, M> {
    pub fn with_grid(self, grid: Grid) -> GeneratorBuilder<Set, M> {
        GeneratorBuilder {
            grid: Some(grid),
            models: self.models,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }
}

impl<G> GeneratorBuilder<G, Unset> {
    pub fn with_models(self, models: Vec<NodeModel>) -> GeneratorBuilder<G, Set> {
        let models = expand_models(models);
        self.with_expanded_models(models)
    }

    pub fn with_expanded_models(self, models: Vec<ExpandedNodeModel>) -> GeneratorBuilder<G, Set> {
        GeneratorBuilder {
            grid: self.grid,
            models: Some(models),
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            typestate: PhantomData,
        }
    }
}

impl<G, M> GeneratorBuilder<G, M> {
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

impl GeneratorBuilder<Set, Set> {
    pub fn build(self) -> Generator {
        let models = self.models.unwrap();
        let models_count = models.len();
        let grid = self.grid.unwrap();
        let direction_count = grid.directions().len();
        let nodes_count = grid.total_size();

        Generator {
            grid,
            max_retry_count: self.max_retry_count,
            node_selection_heuristic: self.node_selection_heuristic,
            model_selection_heuristic: self.model_selection_heuristic,
            rng: thread_rng(),
            nodes: bitvec![1; nodes_count * models_count],
            possible_models_count: vec![models_count, nodes_count],
            propagation_stack: Vec::new(),
            models,
            compatibility_rules: Rc::new(Array::from_elem(
                (models_count, direction_count),
                Vec::new(),
            )),
            supports_count: Array::zeros((nodes_count, models_count, direction_count)),
        }
    }
}
