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
    // pub(crate) sockets: [Vec<SocketId>; 4],
    pub(crate) sockets: Vec<Vec<SocketId>>,
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1.0
    pub(crate) weight: f32,
    /// Allowed rotations of this NodeModel in the output, around the Z axis. Defaults to none.
    /// Note: In 3d, top and bottom sockets of a model should be invariant to rotation around the Z axis.
    pub(crate) allowed_rotations: HashSet<NodeRotation>,
}

impl NodeModel {
    pub fn new_3d<T: Into<Vec<SocketId>>>(
        up: T,
        left: T,
        down: T,
        right: T,
        top: T,
        bottom: T,
    ) -> Self {
        Self {
            sockets: vec![
                up.into(),
                left.into(),
                down.into(),
                right.into(),
                top.into(),
                bottom.into(),
            ],
            allowed_rotations: HashSet::new(),
            weight: 1.0,
        }
    }

    pub fn new_2d<T: Into<Vec<SocketId>>>(up: T, left: T, bottom: T, right: T) -> Self {
        Self {
            sockets: vec![up.into(), left.into(), bottom.into(), right.into()],
            allowed_rotations: HashSet::new(),
            weight: 1.0,
        }
    }

    pub fn with_rotations<T: Into<HashSet<NodeRotation>>>(mut self, rotations: T) -> Self {
        self.allowed_rotations = rotations.into();
        self
    }
    pub fn with_all_rotations(mut self) -> Self {
        self.allowed_rotations = ALL_NODE_ROTATIONS.iter().cloned().collect();
        self
    }
    pub fn with_no_rotations(mut self) -> Self {
        self.allowed_rotations = HashSet::new();
        self
    }

    pub fn with_weight<T: Into<f32>>(mut self, weight: T) -> Self {
        self.weight = weight.into();
        self
    }
}

pub enum Sockets {
    Single(SocketId),
    Multiple(Vec<SocketId>),
}

impl Into<Vec<SocketId>> for Sockets {
    fn into(self) -> Vec<SocketId> {
        match self {
            Sockets::Single(socket) => vec![socket],
            Sockets::Multiple(sockets) => sockets,
        }
    }
}

pub struct ExpandedNodeModel {
    /// Allowed connections for this NodeModel in the output: up, left, bottom, right
    // sockets: [Vec<SocketId>; 4],
    pub(crate) sockets: Vec<Vec<SocketId>>,
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1
    pub(crate) weight: f32,
    /// Index of the NodeModel this was expanded from
    model_index: ModelIndex,
    /// Rotation of the NodeModel in degrees
    rotation: NodeRotation,
}

pub struct GeneratedNode {
    /// Index of the NodeModel this was expanded from
    model_index: ModelIndex,
    /// Rotation of the NodeModel in degrees
    rotation: NodeRotation,
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum NodeRotation {
    Rot0,
    Rot90,
    Rot180,
    Rot270,
}

impl NodeRotation {
    fn value(&self) -> u32 {
        match *self {
            NodeRotation::Rot0 => 0,
            NodeRotation::Rot90 => 90,
            NodeRotation::Rot180 => 180,
            NodeRotation::Rot270 => 270,
        }
    }

    fn index(&self) -> usize {
        match *self {
            NodeRotation::Rot0 => 0,
            NodeRotation::Rot90 => 1,
            NodeRotation::Rot180 => 2,
            NodeRotation::Rot270 => 3,
        }
    }

    pub fn rotate_sockets(&self, sockets: &mut [Vec<SocketId>]) {
        // We only rotate around Z
        sockets[0..4].rotate_right(self.index());
    }
}

pub const ALL_NODE_ROTATIONS: &'static [NodeRotation] = &[
    NodeRotation::Rot90,
    NodeRotation::Rot180,
    NodeRotation::Rot270,
];

#[derive(Clone)]
pub struct Node {
    /// Index of the NodeModel
    model_index: ModelIndex,
    /// Rotation of the NodeModel in degrees
    rotation: NodeRotation,
}

pub struct Nodes {}
