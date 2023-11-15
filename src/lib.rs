mod grid;

use rand::{
    distributions::Distribution, distributions::WeightedIndex, rngs::ThreadRng, thread_rng, Rng,
};
use std::collections::HashSet;

/// Errors that can occur in Ghx_ProcGen
#[derive(thiserror::Error, Debug)]
pub enum ProcGenError {
    #[error("Failed to generate")]
    GenerationFailure(),
}

const DEFAULT_BLOCKS_RETRY_COUNT: u32 = 10;
const MAX_NOISE_VALUE: f32 = 1E-6;

/// Id of a possible connection type
pub type SocketId = u32;
/// Id of a possible connection type
pub type ModelIndex = usize;

pub struct NodeModel {
    /// Allowed connections for this NodeModel in the output: up, left, bottom, right
    sockets: [Vec<SocketId>; 4],
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1
    weight: f32,
    // /// Allowed rotations of this NodeModel in the output: 90°, 180°, 270°. Defaults to false
    // allowed_rotations: [bool; 3],
    /// Allowed rotations of this NodeModel in the output
    allowed_rotations: HashSet<NodeRotation>,
}

impl NodeModel {
    pub fn new(up: SocketId, left: SocketId, bottom: SocketId, right: SocketId) -> Self {
        Self {
            sockets: [vec![up], vec![left], vec![bottom], vec![right]],
            // allowed_rotations: [false, false, false],
            allowed_rotations: HashSet::new(),
            weight: 1.0,
        }
    }
}

pub struct ExpandedNodeModel {
    /// Allowed connections for this NodeModel in the output: up, left, bottom, right
    sockets: [Vec<SocketId>; 4],
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1
    weight: f32,
    /// Index of the NodeModel this was expanded from
    model_index: ModelIndex,
    /// Rotation of the NodeModel in degrees
    rotation: NodeRotation,
}

#[derive(Clone, Copy)]
pub enum NodeRotation {
    Rot90,
    Rot180,
    Rot270,
}

impl NodeRotation {
    fn value(&self) -> u32 {
        match *self {
            NodeRotation::Rot90 => 90,
            NodeRotation::Rot180 => 180,
            NodeRotation::Rot270 => 270,
        }
    }

    fn index(&self) -> usize {
        match *self {
            NodeRotation::Rot90 => 0,
            NodeRotation::Rot180 => 1,
            NodeRotation::Rot270 => 2,
        }
    }

    fn rotate_sockets(&self, sockets: &mut [Vec<SocketId>; 4]) {
        sockets.rotate_right(self.index() + 1);
    }
}

#[derive(Clone)]
pub struct Node {
    /// Index of the NodeModel
    model_index: ModelIndex,
    /// Rotation of the NodeModel in degrees
    rotation: NodeRotation,
}

pub struct Nodes {}

pub fn expand_models(models: Vec<NodeModel>) -> Vec<ExpandedNodeModel> {
    let mut expanded_models = Vec::new();
    for (index, model) in models.iter().enumerate() {
        for rotation in &model.allowed_rotations {
            let mut sockets = model.sockets.clone();
            rotation.rotate_sockets(&mut sockets);
            expanded_models.push(ExpandedNodeModel {
                sockets,
                weight: model.weight,
                model_index: index,
                rotation: *rotation,
            });
        }
    }
    expanded_models
}

pub fn generate(
    models: Vec<ExpandedNodeModel>,
    width: u32,
    height: u32,
    max_iteration: Option<u32>,
) -> Result<Nodes, ProcGenError> {
    let mut rng = thread_rng();
    let all_models_indexes: Vec<ModelIndex> = (0..models.len()).collect();
    // TODO Might change the structure
    let generated_nodes: Vec<Vec<ModelIndex>> = std::iter::repeat(all_models_indexes.clone())
        .take(width as usize * height as usize)
        .collect();
    // TODO max_iteration default value
    let max_iteration = max_iteration.unwrap_or(DEFAULT_BLOCKS_RETRY_COUNT);
    for i in 1..max_iteration {
        // TODO Split generation in multiple blocks
        let success = generate_nodes(&models, &generated_nodes, &mut rng);
        if success {
            println!("Successfully generated");
            break;
        } else {
            println!("Failed to generate, retrying {}/{}", i, max_iteration);
        }
    }
    Err(ProcGenError::GenerationFailure())
}

fn select_node_to_generate<'a>(
    nodes: &'a Vec<Vec<ModelIndex>>,
    rng: &mut ThreadRng,
) -> Option<&'a Vec<ModelIndex>> {
    // Pick a node according to the heuristic
    // TODO Multiple heuristics ? (Entropy, Minimal remaining value)
    let mut min = f32::MAX;
    let mut picked_node = None;
    for (index, node) in nodes.iter().enumerate() {
        // If the node is not generated yet (multiple possibilities)
        if node.len() > 1 {
            // Noise added to entropy so that when evaluating multiples candidates with the same entropy, we pick a random one, not in the evaluating order.
            let noise = MAX_NOISE_VALUE * rng.gen::<f32>();
            if (node.len() as f32) < min {
                min = node.len() as f32 + noise;
                // index_of_min = Some(index);
                picked_node = Some(&nodes[index]);
            }
        }
    }
    picked_node
}

fn generate_nodes(
    models: &Vec<ExpandedNodeModel>,
    nodes: &Vec<Vec<ModelIndex>>,
    rng: &mut ThreadRng,
) -> bool {
    // TODO Check this upper limit
    for i in 1..nodes.len() {
        let selected_node = select_node_to_generate(nodes, rng);
        if let Some(selected_node) = selected_node {
            // We found a node not yet generated
            // Observe/collapse the node: select a model for the node
            // TODO May cache the current sum of weights at each node.
            let weighted_distribution = WeightedIndex::new(
                selected_node
                    .iter()
                    .map(|model_index| models[*model_index].weight),
            )
            .unwrap();
            let selected_model_index = selected_node[weighted_distribution.sample(rng)];

            for model_index in selected_node {
                // TODO Remove possibility
                // TODO Enqueue removal for propagation
            }
        } else {
            // Block fully generated
            return true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_test() {
        let models = vec![
            // corner
            NodeModel::new(0, 0, 1, 1),
            // cross
            NodeModel::new(0, 0, 1, 1),
            // empty
            NodeModel::new(0, 0, 0, 0),
            // line
            NodeModel::new(0, 1, 0, 1),
            // T intersection
            NodeModel::new(0, 1, 1, 1),
        ];
        let expanded = expand_models(models);
        generate(expanded, 8, 8, None);
    }
}
