use bitvec::vec::BitVec;
use ndarray::{Array, Ix3};
use rand::{distributions::Distribution, distributions::WeightedIndex, rngs::ThreadRng, Rng};
use std::rc::Rc;

use crate::{
    grid::{
        direction::{Cartesian2D, DirectionSet},
        Grid,
    },
    ProcGenError,
};

use self::{
    builder::{GeneratorBuilder, Unset},
    node::{ModelIndex, Nodes},
    rules::Rules,
};

pub mod builder;
pub mod node;
pub mod rules;

const MAX_NOISE_VALUE: f32 = 1E-6;

pub enum NodeSelectionHeuristic {
    MinimumRemainingValue,
}

pub enum ModelSelectionHeuristic {
    WeightedProbability,
}

struct PropagationEntry {
    node_index: usize,
    model_index: ModelIndex,
}

pub struct Generator<T: DirectionSet> {
    // Configuration
    grid: Grid<T>,
    rules: Rc<Rules<T>>,
    max_retry_count: u32,
    node_selection_heuristic: NodeSelectionHeuristic,
    model_selection_heuristic: ModelSelectionHeuristic,

    // Internal
    rng: ThreadRng,

    // Generation state
    /// `nodes[node_index * self.rules.models_count() + model_index]` is true (1) if model with index `model_index` is still allowed on node with index `node_index`
    nodes: BitVec<usize>,
    /// Stores how many models are still possible for a given node
    possible_models_count: Vec<usize>,

    // Constraint satisfaction algorithm data
    /// Stack of bans to propagate
    propagation_stack: Vec<PropagationEntry>,
    /// The value at `support_count[node_index][model_index][direction]` represents the number of supports of a `model_index` at `node_index` from `direction`
    supports_count: Array<u32, Ix3>,
}

impl<DS: DirectionSet> Generator<DS> {
    pub fn builder() -> GeneratorBuilder<Unset, Unset, Cartesian2D> {
        GeneratorBuilder::new()
    }

    pub fn generate(&mut self) -> Result<Nodes, ProcGenError> {
        for i in 1..self.max_retry_count {
            // TODO Split generation in multiple blocks
            let success = self.try_generate_all_nodes();
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
        Err(ProcGenError::GenerationFailure)
    }

    fn try_generate_all_nodes(&mut self) -> bool {
        for _i in 1..self.grid.total_size() {
            let selected_node_index = self.select_node_to_generate();
            if let Some(node_index) = selected_node_index {
                // We found a node not yet generated
                // "Observe/collapse" the node: select a model for the node
                let selected_model_index = self.select_model(node_index);

                // Iterate all the possible models because we don't have an easy way to iterate only the models possible at node_index. But we'll filter impossible models right away. TODO: iter_ones ?
                for model_index in 0..self.rules.models_count() {
                    if model_index == selected_model_index {
                        continue;
                    }
                    if !self.is_model_possible(node_index, model_index) {
                        continue;
                    }

                    // Enqueue removal for propagation
                    self.propagation_stack.push(PropagationEntry {
                        node_index,
                        model_index,
                    });

                    // None of these model is possible on this node now, set their support to 0
                    // TODO May not be needed
                    #[cfg(feature = "zeroise-support")]
                    for dir in self.grid.directions() {
                        let supports_count =
                            &mut self.supports_count[(node_index, *model_index, *dir as usize)];
                        *supports_count = 0;
                    }
                }
                // Remove eliminated possibilities, all at once
                // TODO Remove alias ?
                for mut bit in self.nodes[node_index * self.rules.models_count()
                    ..node_index * self.rules.models_count() + self.rules.models_count()]
                    .iter_mut()
                {
                    *bit = false;
                }
                self.nodes.set(
                    node_index * self.rules.models_count() + selected_model_index,
                    true,
                );

                if !self.propagate() {
                    return false;
                }
            } else {
                // Block fully generated
                return true;
            }
        }
        true
    }

    fn propagate(&mut self) -> bool {
        // Clone the Rc to allow for mutability in the interior loops
        let rules = Rc::clone(&self.rules);

        while let Some(from) = self.propagation_stack.pop() {
            let from_position = self.grid.get_position(from.node_index);
            // We want to update all the adjacent nodes (= in all directions)
            for dir in self.grid.directions() {
                // Get the adjacent node in this direction, it may not exist.
                if let Some(to_node_index) = self.grid.get_next_index(&from_position, *dir) {
                    // Decrease the support count of all models previously supported by "from"
                    for &model in rules.supported_models(from.model_index, *dir) {
                        let supports_count =
                            &mut self.supports_count[(to_node_index, model, *dir as usize)];
                        *supports_count -= 1;
                        // When we find a model which is now unsupported, we queue a ban
                        // We check for == because we only want to queue the event once.
                        if *supports_count == 0 {
                            if self.ban_model_from_node(to_node_index, model) {
                                // Failed generation.
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn ban_model_from_node(
        &mut self,
        // node_state: &mut Vec<ModelIndex>,
        node_index: usize,
        model_index: usize,
    ) -> bool {
        // Enqueue removal for propagation
        self.propagation_stack.push(PropagationEntry {
            node_index,
            model_index,
        });
        // Update the supports
        // TODO May not be needed
        #[cfg(feature = "zeroise-support")]
        for dir in self.grid.directions() {
            let supports_count = &mut self.supports_count[(node_index, model_index, *dir as usize)];
            *supports_count = 0;
        }
        // Update the state
        self.nodes
            .set(node_index * self.rules.models_count() + model_index, false);
        self.possible_models_count[node_index] -= 1;
        self.possible_models_count[node_index] == 0
    }

    fn select_node_to_generate<'a>(&mut self) -> Option<usize> {
        // Pick a node according to the heuristic
        match self.node_selection_heuristic {
            NodeSelectionHeuristic::MinimumRemainingValue => {
                let mut min = f32::MAX;
                let mut picked_node = None;
                for (index, &count) in self.possible_models_count.iter().enumerate() {
                    // If the node is not generated yet (multiple possibilities)
                    if count > 1 {
                        // Noise added to entropy so that when evaluating multiples candidates with the same entropy, we pick a random one, not in the evaluating order.
                        let noise = MAX_NOISE_VALUE * self.rng.gen::<f32>();
                        if (count as f32 + noise) < min {
                            min = count as f32 + noise;
                            picked_node = Some(index);
                        }
                    }
                }
                picked_node
            }
        }
    }

    fn select_model(&mut self, node_index: usize) -> usize {
        match self.model_selection_heuristic {
            ModelSelectionHeuristic::WeightedProbability => {
                let possible_models: Vec<ModelIndex> = (0..self.rules.models_count())
                    .filter(|&model_index| self.is_model_possible(node_index, model_index))
                    .collect();

                // TODO May cache the current sum of weights at each node.
                let weighted_distribution = WeightedIndex::new(
                    possible_models
                        .iter()
                        .map(|&model_index| self.rules.weight(model_index)),
                )
                .unwrap();
                possible_models[weighted_distribution.sample(&mut self.rng)]
            }
        }
    }

    #[inline]
    fn is_model_possible(&self, node_index: usize, model_index: usize) -> bool {
        self.nodes[node_index * self.rules.models_count() + model_index] == true
    }
}
