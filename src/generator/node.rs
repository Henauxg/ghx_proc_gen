use std::collections::HashSet;

/// Id of a possible connection type
pub type SocketId = u32;
/// Index of a model
pub type ModelIndex = usize;

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

pub struct NodeModel {
    /// Allowed connections for this NodeModel in the output: up, left, bottom, right
    pub(crate) sockets: [Vec<SocketId>; 4],
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1
    pub(crate) weight: f32,
    // /// Allowed rotations of this NodeModel in the output: 90°, 180°, 270°. Defaults to false
    // allowed_rotations: [bool; 3],
    /// Allowed rotations of this NodeModel in the output
    pub(crate) allowed_rotations: HashSet<NodeRotation>,
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
    pub(crate) weight: f32,
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

    pub fn rotate_sockets(&self, sockets: &mut [Vec<SocketId>; 4]) {
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
