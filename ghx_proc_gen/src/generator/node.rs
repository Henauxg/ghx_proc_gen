use std::{collections::HashSet, marker::PhantomData};

use crate::grid::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet};

/// Id of a possible connection type
pub type SocketId = u32;
/// Index of a model
pub type ModelIndex = usize;

pub(crate) fn expand_models<T: DirectionSet>(
    models: Vec<NodeModel<T>>,
    rotation_axis: Direction,
) -> Vec<ExpandedNodeModel> {
    let mut expanded_models = Vec::new();
    for (index, model) in models.iter().enumerate() {
        // Iterate on a vec of all possible node rotations and filter with the set to have a deterministic insertion order of expanded nodes.
        for rotation in ALL_NODE_ROTATIONS {
            if model.allowed_rotations.contains(&rotation) {
                let sockets = model.rotated_sockets(*rotation, rotation_axis);
                expanded_models.push(ExpandedNodeModel {
                    sockets,
                    weight: model.weight,
                    original_index: index,
                    rotation: *rotation,
                });
            }
        }
    }
    expanded_models
}

/// Represents a model to be used by a [`crate::generator::Generator`] as a "building-block" to fill out the generated area.
pub struct NodeModel<T: DirectionSet> {
    /// Allowed connections for this [`NodeModel`] in the output.
    sockets: Vec<Vec<SocketId>>,
    /// Weight factor influencing the density of this [`NodeModel`] in the generated output.
    ///
    ///  Defaults to 1.0
    weight: f32,
    /// Allowed rotations of this [`NodeModel`] in the output, around the rotation axis specified in the rules.
    ///
    /// Defaults to only [`NodeRotation::Rot0`].
    ///
    /// Note:
    /// - In 3d, top and bottom sockets of a model should be invariant to rotation around the chosen rotation axis.
    /// - In 2d, the rotation axis cannot be modified and is set to [`Direction::ZForward`].
    allowed_rotations: HashSet<NodeRotation>,

    typestate: PhantomData<T>,
}

/// Sockets for a model to be used in a 2d cartesian grid.
pub enum SocketsCartesian2D {
    /// The model has only 1 socket, and its is the same in all directions.
    Mono(SocketId),
    /// The model has 1 socket per side.
    Simple {
        x_pos: SocketId,
        x_neg: SocketId,
        y_pos: SocketId,
        y_neg: SocketId,
    },
    /// The model has multiple sockets per side.
    Multiple {
        x_pos: Vec<SocketId>,
        x_neg: Vec<SocketId>,
        y_pos: Vec<SocketId>,
        y_neg: Vec<SocketId>,
    },
}

impl Into<Vec<Vec<SocketId>>> for SocketsCartesian2D {
    fn into(self) -> Vec<Vec<SocketId>> {
        match self {
            SocketsCartesian2D::Mono(socket) => vec![vec![socket]; 4],
            SocketsCartesian2D::Simple {
                x_pos,
                y_pos,
                x_neg,
                y_neg,
            } => {
                vec![vec![x_pos], vec![y_pos], vec![x_neg], vec![y_neg]]
            }
            SocketsCartesian2D::Multiple {
                x_pos,
                y_pos,
                x_neg,
                y_neg,
            } => {
                vec![x_pos, y_pos, x_neg, y_neg]
            }
        }
    }
}

impl SocketsCartesian2D {
    /// Create a [`NodeModel`] from its sockets definition, with default values for the other members.
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
    /// Create a [`NodeModel`] from a [`SocketsCartesian2D`] definition, with default values for the other members.
    pub fn new_cartesian_2d(sockets: SocketsCartesian2D) -> NodeModel<Cartesian2D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([NodeRotation::Rot0]),
            weight: 1.0,
            typestate: PhantomData,
        }
    }
}

/// Sockets for a model to be used in a 3d cartesian grid.
pub enum SocketsCartesian3D {
    /// The model has only 1 socket, and its is the same in all directions.
    Mono(SocketId),
    /// The model has 1 socket per side.
    Simple {
        x_pos: SocketId,
        x_neg: SocketId,
        z_pos: SocketId,
        z_neg: SocketId,
        y_pos: SocketId,
        y_neg: SocketId,
    },
    /// The model has multiple sockets per side.
    Multiple {
        x_pos: Vec<SocketId>,
        x_neg: Vec<SocketId>,
        z_pos: Vec<SocketId>,
        z_neg: Vec<SocketId>,
        y_pos: Vec<SocketId>,
        y_neg: Vec<SocketId>,
    },
}

impl Into<Vec<Vec<SocketId>>> for SocketsCartesian3D {
    fn into(self) -> Vec<Vec<SocketId>> {
        match self {
            SocketsCartesian3D::Mono(socket) => vec![vec![socket]; 6],
            SocketsCartesian3D::Simple {
                x_pos,
                y_pos,
                x_neg,
                y_neg,
                z_pos,
                z_neg,
            } => {
                vec![
                    vec![x_pos],
                    vec![y_pos],
                    vec![x_neg],
                    vec![y_neg],
                    vec![z_pos],
                    vec![z_neg],
                ]
            }
            SocketsCartesian3D::Multiple {
                x_pos,
                y_pos,
                x_neg,
                y_neg,
                z_pos,
                z_neg,
            } => {
                vec![x_pos, y_pos, x_neg, y_neg, z_pos, z_neg]
            }
        }
    }
}

impl SocketsCartesian3D {
    /// Create a [`NodeModel`] from its sockets definition, with default values for the other members: weight is 1.0 and the model will not be rotated.
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
    /// Create a [`NodeModel`] from a [`SocketsCartesian3D`] definition, with default values for the other members: weight is 1.0 and the model will not be rotated.
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
    /// Specify that this [`NodeModel`] can be rotated in exactly one way: `rotation` (in addition to the default [`NodeRotation::Rot0`] rotation)
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotation(mut self, rotation: NodeRotation) -> Self {
        self.allowed_rotations = HashSet::from([NodeRotation::Rot0, rotation]);
        self
    }
    /// Specify that this [`NodeModel`] can be rotated in every way specified in `rotations`, (in addition to the default [`NodeRotation::Rot0`] rotation)
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotations<R: Into<HashSet<NodeRotation>>>(mut self, rotations: R) -> Self {
        self.allowed_rotations = rotations.into();
        self.allowed_rotations.insert(NodeRotation::Rot0);
        self
    }
    /// Specify that this [`NodeModel`] can be rotated in every way (in addition to the default [`NodeRotation::Rot0`] rotation)
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_all_rotations(mut self) -> Self {
        self.allowed_rotations = ALL_NODE_ROTATIONS.iter().cloned().collect();
        self
    }
    /// Specify that this [`NodeModel`] can not be rotated in any way except the default [`NodeRotation::Rot0`] rotation
    pub fn with_no_rotations(mut self) -> Self {
        self.allowed_rotations = HashSet::from([NodeRotation::Rot0]);
        self
    }
    /// Specify this [`NodeModel`] weight. Used by a [`Generator`] when using [`ModelSelectionHeuristic::WeightedProbability`]. All the variations(rotations) of this [`NodeModel`] will use the same weight.
    pub fn with_weight<W: Into<f32>>(mut self, weight: W) -> Self {
        self.weight = weight.into();
        self
    }

    fn rotated_sockets(&self, rotation: NodeRotation, axis: Direction) -> Vec<Vec<SocketId>> {
        let mut rotated_sockets = vec![Vec::new(); self.sockets.len()];

        // Not pretty: if the node sockets contain the rotation axis, add the unmodified sockets to the rotated_sockets.
        if self.sockets.len() > axis as usize {
            for fixed_axis in [axis, axis.opposite()] {
                rotated_sockets[fixed_axis as usize].extend(&self.sockets[fixed_axis as usize]);
            }
        }

        let basis = axis.rotation_basis();
        let mut rotated_basis = basis.to_vec();
        rotated_basis.rotate_right(rotation.index());

        for i in 0..basis.len() {
            rotated_sockets[basis[i] as usize].extend(&self.sockets[rotated_basis[i] as usize]);
        }
        rotated_sockets
    }
}

#[derive(Debug)]
pub struct ExpandedNodeModel {
    /// Allowed connections for this [`NodeModel`] in the output
    sockets: Vec<Vec<SocketId>>,
    /// Weight factor influencing the density of this [`NodeModel`] in the generated output. Defaults to 1
    weight: f32,
    /// Index of the [`NodeModel`] this was expanded from
    original_index: ModelIndex,
    /// Rotation of the [`NodeModel`]
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
            model_index: self.original_index,
            rotation: self.rotation,
        }
    }
}

/// Output of a [`Generator`]. Used to identify a specific variation of an input model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeneratedNode {
    /// Index of the [`NodeModel`] this was expanded from
    pub model_index: ModelIndex,
    /// Rotation of the [`NodeModel`]
    pub rotation: NodeRotation,
}

/// Represents a rotation around an Axis, in the trigonometric(counterclockwise) direction
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum NodeRotation {
    Rot0,
    Rot90,
    Rot180,
    Rot270,
}

impl NodeRotation {
    /// Returns the value of the rotation in °(degrees).
    pub fn value(&self) -> u32 {
        match *self {
            NodeRotation::Rot0 => 0,
            NodeRotation::Rot90 => 90,
            NodeRotation::Rot180 => 180,
            NodeRotation::Rot270 => 270,
        }
    }
    /// Returns the index of the enum member in the enumeration.
    pub fn index(&self) -> usize {
        match *self {
            NodeRotation::Rot0 => 0,
            NodeRotation::Rot90 => 1,
            NodeRotation::Rot180 => 2,
            NodeRotation::Rot270 => 3,
        }
    }
}

/// All the possible rotations for a [`NodeModel`]
pub const ALL_NODE_ROTATIONS: &'static [NodeRotation] = &[
    NodeRotation::Rot0,
    NodeRotation::Rot90,
    NodeRotation::Rot180,
    NodeRotation::Rot270,
];
