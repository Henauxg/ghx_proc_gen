use std::collections::HashSet;

/// Errors that can occur in Ghx_ProcGen
#[derive(thiserror::Error, Debug)]
pub enum ProcGenError {
    #[error("Failed to generate")]
    GenerationFailure(),
}

const DEFAULT_BLOCKS_RETRY_COUNT: u32 = 10;

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
    let mut all_possibilities = HashSet::new();
    for i in 0..models.len() {
        all_possibilities.insert(i);
    }
    let nodes: Vec<HashSet<ModelIndex>> = std::iter::repeat(all_possibilities.clone())
        .take(width as usize * height as usize)
        .collect();
    // TODO max_iteration default value

    let max_iteration = max_iteration.unwrap_or(DEFAULT_BLOCKS_RETRY_COUNT);
    for i in 1..max_iteration {
        // TODO Split generation in multiple blocks
        let success = generate_block(&nodes);
        if success {
            println!("Successfully generated a block");
            break;
        } else {
            println!(
                "Failed to generate a block, retrying {}/{}",
                i, max_iteration
            );
        }
    }
    Err(ProcGenError::GenerationFailure())
}

fn generate_block(nodes: &Vec<HashSet<ModelIndex>>) -> bool {
    // TODO Pick a node with minimal entropy
    // TODO Observe the node: pick a model for the node
    // TODO Propagate the constraints
    false
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
