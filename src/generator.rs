use crate::{grid::Grid, ProcGenError};
use rand::{distributions::Distribution, distributions::WeightedIndex, rngs::ThreadRng, Rng};

use self::{
    builder::{GeneratorBuilder, Unset},
    node::{ExpandedNodeModel, ModelIndex, Nodes},
};

pub mod builder;
pub mod node;

const MAX_NOISE_VALUE: f32 = 1E-6;

pub enum NodeSelectionHeuristic {
    MinimumRemainingValue,
}

struct PropagationEntry {
    node_index: usize,
    model_index: ModelIndex,
}

pub struct Generator {
    // Configuration
    models: Vec<ExpandedNodeModel>,
    grid: Grid,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    // Internal
    rng: ThreadRng,
    // Solver
    propagation_stack: Vec<PropagationEntry>,
}

impl Generator {
    pub fn builder() -> GeneratorBuilder<Unset, Unset> {
        GeneratorBuilder::new()
    }

    pub fn generate(&mut self) -> Result<Nodes, ProcGenError> {
        let all_models_indexes: Vec<ModelIndex> = (0..self.models.len()).collect();
        // TODO Might change the structure
        let mut generated_nodes: Vec<Vec<ModelIndex>> =
            std::iter::repeat(all_models_indexes.clone())
                .take(self.grid.total_size())
                .collect();
        for i in 1..self.max_retry_count {
            // TODO Split generation in multiple blocks
            let success = self.try_generate_all_nodes(&mut generated_nodes);
            if success {
                println!("Successfully generated");
                break;
            } else {
                println!(
                    "Failed to generate, retrying {}/{}",
                    i, self.max_retry_count
                );
            }
        }
        Err(ProcGenError::GenerationFailure())
    }

    fn try_generate_all_nodes(&mut self, nodes: &mut Vec<Vec<ModelIndex>>) -> bool {
        // TODO Check this upper limit
        for i in 1..nodes.len() {
            let selected_node_index = self.select_node_to_generate(nodes);
            if let Some(node_index) = selected_node_index {
                let selected_node = &mut nodes[node_index];
                // We found a node not yet generated
                // Observe/collapse the node: select a model for the node
                // TODO May cache the current sum of weights at each node.
                let weighted_distribution = WeightedIndex::new(
                    selected_node
                        .iter()
                        .map(|model_index| self.models[*model_index].weight),
                )
                .unwrap();
                let selected_model_index =
                    selected_node[weighted_distribution.sample(&mut self.rng)];

                // TODO Remove possibility
                selected_node.clear();
                selected_node.push(selected_model_index);
                for model_index in selected_node {
                    if *model_index == selected_model_index {
                        continue;
                    }
                    // self.ban_model_from_node(selected_node, model_index);
                    for direction in self.grid.directions() {
                        // TODO Update supports
                    }
                    // Enqueue removal for propagation
                    self.propagation_stack.push(PropagationEntry {
                        node_index,
                        model_index: *model_index,
                    });
                }

                // Propagate
                while let Some(propagation_entry) = self.propagation_stack.pop() {
                    let grid_position = self.grid.get_position(propagation_entry.node_index);
                    for direction in self.grid.directions() {
                        if let Some(neighbour) =
                            self.grid.get_next_index(&grid_position, *direction)
                        {
                            // TODO
                        }
                    }
                }
            } else {
                // Block fully generated
                return true;
            }
        }
        true
    }

    fn ban_model_from_node(&self, selected_node: &[usize], model_index: usize) -> bool {
        todo!()
    }

    fn select_node_to_generate<'a>(&mut self, nodes: &'a Vec<Vec<ModelIndex>>) -> Option<usize> {
        // Pick a node according to the heuristic
        // TODO Add heuristics (Entropy, Scanline, ...)
        match self.node_selection_heuristic {
            NodeSelectionHeuristic::MinimumRemainingValue => {
                let mut min = f32::MAX;
                let mut picked_node = None;
                for (index, node) in nodes.iter().enumerate() {
                    // If the node is not generated yet (multiple possibilities)
                    if node.len() > 1 {
                        // Noise added to entropy so that when evaluating multiples candidates with the same entropy, we pick a random one, not in the evaluating order.
                        let noise = MAX_NOISE_VALUE * self.rng.gen::<f32>();
                        if (node.len() as f32) < min {
                            min = node.len() as f32 + noise;
                            picked_node = Some(index);
                        }
                    }
                }
                picked_node
            }
        }
    }
}
