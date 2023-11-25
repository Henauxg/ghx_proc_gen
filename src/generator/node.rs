use std::{collections::HashSet, marker::PhantomData};

use crate::grid::direction::{Cartesian2D, Cartesian3D, DirectionSet};

/// Id of a possible connection type
pub type SocketId = u32;
/// Index of a model
pub type ModelIndex = usize;

pub(crate) fn expand_models<T: DirectionSet>(models: Vec<NodeModel<T>>) -> Vec<ExpandedNodeModel> {
    let mut expanded_models = Vec::new();
    for (index, model) in models.iter().enumerate() {
        for rotation in &model.allowed_rotations {
            let mut sockets = model.sockets.clone();
            rotation.rotate_sockets(&mut sockets);
            expanded_models.push(ExpandedNodeModel {
                sockets,
                weight: model.weight,
                original_index: index,
                rotation: *rotation,
            });
        }
    }
    expanded_models
}

pub struct NodeModel<T: DirectionSet> {
    /// Allowed connections for this NodeModel in the output: up, left, bottom, right
    sockets: Vec<Vec<SocketId>>,
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1.0
    weight: f32,
    /// Allowed rotations of this NodeModel in the output, around the Z axis. Defaults to only Rot0.
    ///
    /// Note: In 3d, top and bottom sockets of a model should be invariant to rotation around the Z axis.
    allowed_rotations: HashSet<NodeRotation>,

    typestate: PhantomData<T>,
}

pub enum SocketsCartesian2D {
    Mono(SocketId),
    Simple(SocketId, SocketId, SocketId, SocketId),
    Multiple(Vec<SocketId>, Vec<SocketId>, Vec<SocketId>, Vec<SocketId>),
}

impl Into<Vec<Vec<SocketId>>> for SocketsCartesian2D {
    fn into(self) -> Vec<Vec<SocketId>> {
        match self {
            SocketsCartesian2D::Mono(socket) => vec![vec![socket]; 4],
            SocketsCartesian2D::Simple(up, left, down, right) => {
                vec![vec![up], vec![left], vec![down], vec![right]]
            }
            SocketsCartesian2D::Multiple(up, left, down, right) => vec![up, left, down, right],
        }
    }
}

impl SocketsCartesian2D {
    pub fn new_model(self) -> NodeModel<Cartesian2D> {
        NodeModel {
            sockets: self.into(),
            allowed_rotations: HashSet::from([NodeRotation::Rot0]),
            weight: 1.0,
            typestate: PhantomData,
        }
    }
}

impl NodeModel<Cartesian2D> {
    pub fn new_cartesian_2d(sockets: SocketsCartesian2D) -> NodeModel<Cartesian2D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([NodeRotation::Rot0]),
            weight: 1.0,
            typestate: PhantomData,
        }
    }
}

pub enum SocketsCartesian3D {
    Mono(SocketId),
    Simple(SocketId, SocketId, SocketId, SocketId, SocketId, SocketId),
    Multiple(
        Vec<SocketId>,
        Vec<SocketId>,
        Vec<SocketId>,
        Vec<SocketId>,
        Vec<SocketId>,
        Vec<SocketId>,
    ),
}

impl Into<Vec<Vec<SocketId>>> for SocketsCartesian3D {
    fn into(self) -> Vec<Vec<SocketId>> {
        match self {
            SocketsCartesian3D::Mono(socket) => vec![vec![socket]; 6],
            SocketsCartesian3D::Simple(up, left, down, right, top, bottom) => {
                vec![
                    vec![up],
                    vec![left],
                    vec![down],
                    vec![right],
                    vec![top],
                    vec![bottom],
                ]
            }
            SocketsCartesian3D::Multiple(up, left, down, right, top, bottom) => {
                vec![up, left, down, right, top, bottom]
            }
        }
    }
}

impl SocketsCartesian3D {
    pub fn new_model(self) -> NodeModel<Cartesian3D> {
        NodeModel {
            sockets: self.into(),
            allowed_rotations: HashSet::from([NodeRotation::Rot0]),
            weight: 1.0,
            typestate: PhantomData,
        }
    }
}

impl NodeModel<Cartesian3D> {
    pub fn new_cartesian_3d(sockets: SocketsCartesian3D) -> NodeModel<Cartesian3D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([NodeRotation::Rot0]),
            weight: 1.0,
            typestate: PhantomData,
        }
    }
}

impl<T: DirectionSet> NodeModel<T> {
    pub fn with_rotation(mut self, rotation: NodeRotation) -> Self {
        self.allowed_rotations = HashSet::from([NodeRotation::Rot0, rotation]);
        self
    }
    pub fn with_rotations<R: Into<HashSet<NodeRotation>>>(mut self, rotations: R) -> Self {
        self.allowed_rotations = rotations.into();
        self.allowed_rotations.insert(NodeRotation::Rot0);
        self
    }
    pub fn with_all_rotations(mut self) -> Self {
        self.allowed_rotations = ALL_NODE_ROTATIONS.iter().cloned().collect();
        self
    }
    pub fn with_no_rotations(mut self) -> Self {
        self.allowed_rotations = HashSet::from([NodeRotation::Rot0]);
        self
    }

    pub fn with_weight<W: Into<f32>>(mut self, weight: W) -> Self {
        self.weight = weight.into();
        self
    }
}

#[derive(Debug)]
pub struct ExpandedNodeModel {
    /// Allowed connections for this NodeModel in the output
    sockets: Vec<Vec<SocketId>>,
    /// Weight factor between 0 and 1 influencing the density of this NodeModel in the generated output. Defaults to 1
    weight: f32,
    /// Index of the NodeModel this was expanded from
    original_index: ModelIndex,
    /// Rotation of the NodeModel in degrees
    rotation: NodeRotation,
}

impl ExpandedNodeModel {
    pub fn sockets(&self) -> &Vec<Vec<SocketId>> {
        &self.sockets
    }
    pub fn weight(&self) -> f32 {
        self.weight
    }
    pub fn original_index(&self) -> ModelIndex {
        self.original_index
    }
    pub fn rotation(&self) -> NodeRotation {
        self.rotation
    }

    pub(crate) fn to_generated(&self) -> GeneratedNode {
        GeneratedNode {
            index: self.original_index,
            rotation: self.rotation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedNode {
    /// Index of the NodeModel this was expanded from
    pub index: ModelIndex,
    /// Rotation of the NodeModel
    pub rotation: NodeRotation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum NodeRotation {
    Rot0,
    Rot90,
    Rot180,
    Rot270,
}

impl NodeRotation {
    pub fn value(&self) -> u32 {
        match *self {
            NodeRotation::Rot0 => 0,
            NodeRotation::Rot90 => 90,
            NodeRotation::Rot180 => 180,
            NodeRotation::Rot270 => 270,
        }
    }

    pub fn index(&self) -> usize {
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
    NodeRotation::Rot0,
    NodeRotation::Rot90,
    NodeRotation::Rot180,
    NodeRotation::Rot270,
];
