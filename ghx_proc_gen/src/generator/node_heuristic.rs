use rand::{rngs::StdRng, Rng};

use crate::grid::{direction::CoordinateSystem, NodeIndex};

use super::rules::Rules;

/// Defines a heuristic for the choice of a node to generate. For some given Rules, each heuristic will lead to different visual results and different failure rates.
#[derive(Copy, Clone, Debug)]
pub enum NodeSelectionHeuristic {
    /// The node with with the minimum count of possible models remaining will be chosen at each selection iteration. If multiple nodes have the same value, a random one is picked.
    ///s
    /// Similar to `MinimumEntropy` when the models have all more or less the same weight.
    MinimumRemainingValue,
    /// The node with the minimum Shannon entropy (computed from the models weights) will be chosen at each selection iteration. If multiple nodes have the same value, a random one is picked.
    ///
    ///  Similar to `MinimumRemainingValue` when the models have all more or less the same weight.
    MinimumEntropy,
    /// A random node with no special features (except not being generated yet) will be chosen at each selection iteration.
    ///
    /// Often causes a **very high generation failure rate**, except for very simple rules.
    Random,
}

const MAX_NOISE_VALUE: f32 = 1E-2;

/// Defines a heuristic for the choice of a node to generate.
pub(crate) enum InternalNodeSelectionHeuristic {
    MinimumRemainingValue,
    MinimumEntropy {
        /// Initial value of entropy data for any node
        initial_node_entropy_data: NodeEntropyData,
        /// Current entropy data for a given node
        node_entropies: Vec<NodeEntropyData>,
        /// Value of `weight * log(weight)` for a given model
        models_weight_log_weights: Vec<f32>,
    },
    Random,
}

#[derive(Clone, Copy)]
pub(crate) struct NodeEntropyData {
    /// Shannon entropy of the node
    entropy: f32,
    /// Sum of the weights of the models still possible on the node
    weight_sum: f32,
    /// Sum of `weight * log(weight)` of the models still possible on the node
    weight_log_weight_sum: f32,
}

impl NodeEntropyData {
    fn new(weight_sum: f32, weight_log_weight_sum: f32) -> Self {
        Self {
            entropy: entropy(weight_sum, weight_log_weight_sum),
            weight_sum,
            weight_log_weight_sum,
        }
    }

    pub(crate) fn entropy(&self) -> f32 {
        self.entropy
    }
}

fn entropy(weight_sum: f32, weight_log_weight_sum: f32) -> f32 {
    f32::ln(weight_sum) - weight_log_weight_sum / weight_sum
}

impl InternalNodeSelectionHeuristic {
    pub(crate) fn from_external<T: CoordinateSystem + Clone>(
        heuristic: NodeSelectionHeuristic,
        rules: &Rules<T>,
        node_count: usize,
    ) -> Self {
        match heuristic {
            NodeSelectionHeuristic::MinimumRemainingValue => {
                InternalNodeSelectionHeuristic::MinimumRemainingValue
            }
            NodeSelectionHeuristic::Random => InternalNodeSelectionHeuristic::Random,
            NodeSelectionHeuristic::MinimumEntropy => {
                InternalNodeSelectionHeuristic::new_minimum_entropy(rules, node_count)
            }
        }
    }

    fn new_minimum_entropy<T: CoordinateSystem + Clone>(
        rules: &Rules<T>,
        node_count: usize,
    ) -> InternalNodeSelectionHeuristic {
        let mut models_weight_log_weights = Vec::with_capacity(rules.models_count());
        let mut all_models_weight_sum = 0.;
        let mut all_models_weight_log_weight_sum = 0.;
        for model_index in 0..rules.models_count() {
            let weight = rules.weight_unchecked(model_index);
            let weight_log_weight = weight * f32::ln(weight);
            models_weight_log_weights.push(weight_log_weight);
            all_models_weight_sum += weight;
            all_models_weight_log_weight_sum += weight_log_weight;
        }

        let initial_node_entropy_data =
            NodeEntropyData::new(all_models_weight_sum, all_models_weight_log_weight_sum);
        InternalNodeSelectionHeuristic::MinimumEntropy {
            initial_node_entropy_data,
            node_entropies: vec![initial_node_entropy_data; node_count],
            models_weight_log_weights,
        }
    }

    pub(crate) fn reinitialize(&mut self) {
        match self {
            InternalNodeSelectionHeuristic::MinimumEntropy {
                initial_node_entropy_data,
                node_entropies,
                models_weight_log_weights: _,
            } => {
                // `models_weight_log_weights` does not change. We just reset the nodes
                for node_entropy in node_entropies {
                    *node_entropy = *initial_node_entropy_data;
                }
            }
            _ => (),
        }
    }

    pub(crate) fn handle_ban(&mut self, node_index: NodeIndex, model_index: usize, weight: f32) {
        match self {
            InternalNodeSelectionHeuristic::MinimumEntropy {
                initial_node_entropy_data: _,
                node_entropies,
                models_weight_log_weights,
            } => {
                let node_entropy = &mut node_entropies[node_index];
                node_entropy.weight_sum -= weight;
                node_entropy.weight_log_weight_sum -= models_weight_log_weights[model_index];
                node_entropy.entropy =
                    entropy(node_entropy.weight_sum, node_entropy.weight_log_weight_sum)
            }
            _ => (),
        }
    }

    /// Picks a node according to the heuristic
    pub(crate) fn select_node(
        &self,
        possible_models_counts: &Vec<usize>,
        rng: &mut StdRng,
    ) -> Option<NodeIndex> {
        match self {
            InternalNodeSelectionHeuristic::MinimumRemainingValue => {
                let mut min = f32::MAX;
                let mut picked_node = None;
                for (index, &possibilities_count) in possible_models_counts.iter().enumerate() {
                    // If the node is not generated yet (multiple possibilities)
                    if possibilities_count > 1 {
                        // Noise added to models count so that when evaluating multiples candidates with the same value, we pick a random one, not in the evaluation order.
                        let noise = MAX_NOISE_VALUE * rng.gen::<f32>();
                        if (possibilities_count as f32 + noise) < min {
                            min = possibilities_count as f32 + noise;
                            picked_node = Some(index);
                        }
                    }
                }
                picked_node
            }
            InternalNodeSelectionHeuristic::MinimumEntropy {
                initial_node_entropy_data: _,
                node_entropies,
                models_weight_log_weights: _,
            } => {
                let mut min = f32::MAX;
                let mut picked_node = None;
                for (index, &possibilities_count) in possible_models_counts.iter().enumerate() {
                    let entropy = node_entropies[index].entropy();
                    if possibilities_count > 1 && entropy < min {
                        let noise = MAX_NOISE_VALUE * rng.gen::<f32>();
                        if (entropy + noise) < min {
                            min = entropy + noise;
                            picked_node = Some(index);
                        }
                    }
                }
                picked_node
            }
            InternalNodeSelectionHeuristic::Random => {
                let mut picked_node = None;
                let mut candidates = Vec::new();
                for (index, &possibilities_count) in possible_models_counts.iter().enumerate() {
                    if possibilities_count > 1 {
                        candidates.push(index);
                    }
                }
                if candidates.len() > 0 {
                    picked_node = Some(candidates[rng.gen_range(0..candidates.len())]);
                }
                picked_node
            }
        }
    }
}
