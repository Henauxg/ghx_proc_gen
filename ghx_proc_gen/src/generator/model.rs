use std::{collections::HashSet, marker::PhantomData};

#[cfg(feature = "debug-traces")]
use {core::fmt, tracing::warn};

use crate::grid::direction::{Cartesian2D, Cartesian3D, CoordinateSystem, Direction};

use super::{
    rules::CARTESIAN_2D_ROTATION_AXIS,
    socket::{Socket, SocketId, SocketsCartesian2D, SocketsCartesian3D},
};

/// Index of a model
pub type ModelIndex = usize;

/// Represents a model to be used by a [`crate::generator::Generator`] as a "building-block" to fill out the generated area.
#[derive(Clone)]
pub struct Model<T: CoordinateSystem> {
    /// Allowed connections for this [`Model`] in the output.
    sockets: Vec<Vec<Socket>>,
    /// Weight factor influencing the density of this [`Model`] in the generated output.
    ///
    ///  Defaults to 1.0
    weight: f32,
    /// Allowed rotations of this [`Model`] in the output, around the rotation axis specified in the rules.
    ///
    /// Defaults to only [`ModelRotation::Rot0`].
    ///
    /// Notes:
    /// - In 3d, sockets of a model that are on the rotation axis are rotated into new sockets when the model itself is rotated. See [`SocketCollection`] for how to define and/or constrain sockets connections on the rotation axis.
    /// - In 2d, the rotation axis cannot be modified and is set to [`Direction::ZForward`].
    allowed_rotations: HashSet<ModelRotation>,

    /// Name given to this model for debug purposes.
    name: Option<&'static str>,

    typestate: PhantomData<T>,
}

impl Model<Cartesian2D> {
    /// Create a [`Model`] from a [`SocketsCartesian2D`] definition, with default values for the other members.
    pub fn new_cartesian_2d(sockets: SocketsCartesian2D) -> Model<Cartesian2D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([ModelRotation::Rot0]),
            weight: 1.0,
            name: None,
            typestate: PhantomData,
        }
    }

    /// Returns a clone of the [`Model`] with its sockets rotated by `rotation` around [`CARTESIAN_2D_ROTATION_AXIS`].
    pub fn rotated(&self, rotation: ModelRotation) -> Self {
        Self {
            sockets: self.rotated_sockets(rotation, CARTESIAN_2D_ROTATION_AXIS),
            weight: self.weight,
            allowed_rotations: self.allowed_rotations.clone(),
            name: self.name.clone(),
            typestate: PhantomData,
        }
    }
}

impl Model<Cartesian3D> {
    /// Create a [`Model`] from a [`SocketsCartesian3D`] definition, with default values for the other members: weight is 1.0 and the model will not be rotated.
    pub fn new_cartesian_3d(sockets: SocketsCartesian3D) -> Model<Cartesian3D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([ModelRotation::Rot0]),
            weight: 1.0,
            name: None,
            typestate: PhantomData,
        }
    }

    /// Returns a clone of the [`Model`] with its sockets rotated by `rotation` around `axis`.
    pub fn rotated(&self, rotation: ModelRotation, axis: Direction) -> Self {
        Self {
            sockets: self.rotated_sockets(rotation, axis),
            weight: self.weight,
            allowed_rotations: self.allowed_rotations.clone(),
            name: self.name.clone(),
            typestate: PhantomData,
        }
    }
}

impl<T: CoordinateSystem> Model<T> {
    /// Specify that this [`Model`] can be rotated in exactly one way: `rotation` (in addition to the default [`ModelRotation::Rot0`] rotation)
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotation(mut self, rotation: ModelRotation) -> Self {
        self.allowed_rotations = HashSet::from([ModelRotation::Rot0, rotation]);
        self
    }
    /// Specify that this [`Model`] can be rotated in every way specified in `rotations`, (in addition to the default [`ModelRotation::Rot0`] rotation)
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotations<R: Into<HashSet<ModelRotation>>>(mut self, rotations: R) -> Self {
        self.allowed_rotations = rotations.into();
        self.allowed_rotations.insert(ModelRotation::Rot0);
        self
    }
    /// Specify that this [`Model`] can be rotated in every way (in addition to the default [`ModelRotation::Rot0`] rotation)
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_all_rotations(mut self) -> Self {
        self.allowed_rotations = ALL_NODE_ROTATIONS.iter().cloned().collect();
        self
    }
    /// Specify that this [`Model`] can not be rotated in any way except the default [`ModelRotation::Rot0`] rotation
    pub fn with_no_rotations(mut self) -> Self {
        self.allowed_rotations = HashSet::from([ModelRotation::Rot0]);
        self
    }
    /// Specify this [`Model`] weight. The `weight` value should be strictly superior to `0`. If it is not the case, the value will be overriden by `f32::MIN_POSITIVE`.
    ///
    /// Used by a [`super::Generator`] when using [`super::ModelSelectionHeuristic::WeightedProbability`].
    ///
    /// All the variations (rotations) of this [`Model`] will use the same weight.
    pub fn with_weight<W: Into<f32>>(mut self, weight: W) -> Self {
        let mut checked_weight = weight.into();
        if checked_weight <= 0. {
            #[cfg(feature = "debug-traces")]
            warn!(
                "Model with name {:?}, had an invalid weight {} <= 0., weight overriden to f32::MIN: {}",
                self.name, checked_weight, f32::MIN_POSITIVE
            );
            checked_weight = f32::MIN_POSITIVE
        };
        self.weight = checked_weight;
        self
    }

    #[allow(unused_mut)]
    /// Register the given name for this model.
    ///
    /// Does nothing if the `debug-traces` feature is not enabled.
    pub fn with_name(mut self, _name: &'static str) -> Self {
        #[cfg(feature = "debug-traces")]
        {
            self.name = Some(_name);
        }

        self
    }

    fn rotated_sockets(&self, rotation: ModelRotation, rot_axis: Direction) -> Vec<Vec<Socket>> {
        let mut rotated_sockets = vec![Vec::new(); self.sockets.len()];

        // Not pretty: if the node sockets contain the rotation axis
        if self.sockets.len() > rot_axis as usize {
            // Sockets on the rotation axis are marked as rotated
            for fixed_axis in [rot_axis, rot_axis.opposite()] {
                rotated_sockets[fixed_axis as usize]
                    .extend(self.sockets[fixed_axis as usize].clone());
                for socket in &mut rotated_sockets[fixed_axis as usize] {
                    socket.rotate(rotation);
                }
            }
        }

        let basis = rot_axis.rotation_basis();
        let mut rotated_basis = basis.to_vec();
        rotated_basis.rotate_right(rotation.index() as usize);

        for i in 0..basis.len() {
            rotated_sockets[basis[i] as usize]
                .extend(self.sockets[rotated_basis[i] as usize].clone());
        }
        rotated_sockets
    }
}

/// This is a variation of a user [`Model`] generated by the [`crate::generator::Rules`]. One [`Model`] may be transformed into one ore more [`ExpandedModel`] depending on the number of allowed rotations of the model.
#[derive(Debug)]
pub struct ExpandedModel {
    /// Allowed connections for this [`Model`] in the output
    sockets: Vec<Vec<SocketId>>,
    /// Weight factor influencing the density of this [`Model`] in the generated output. Defaults to 1
    weight: f32,
    /// Index of the [`Model`] this was expanded from
    original_index: ModelIndex,
    /// Rotation of the [`Model`]
    rotation: ModelRotation,

    /// Debug name for this model
    #[cfg(feature = "debug-traces")]
    pub name: Option<&'static str>,
}

impl ExpandedModel {
    /// Return the sockets of the model
    pub fn sockets(&self) -> &Vec<Vec<SocketId>> {
        &self.sockets
    }
    /// Returns the weight of the model
    pub fn weight(&self) -> f32 {
        self.weight
    }
    /// Returns the index of the [`Model`] this model was expanded from
    pub fn original_index(&self) -> ModelIndex {
        self.original_index
    }
    /// Returns the rotation applied to the original [``Model`] this model was expanded from
    pub fn rotation(&self) -> ModelRotation {
        self.rotation
    }

    pub(crate) fn to_instance(&self) -> ModelInstance {
        ModelInstance {
            model_index: self.original_index,
            rotation: self.rotation,
        }
    }
}

#[cfg(feature = "debug-traces")]
impl fmt::Display for ExpandedModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{} ({:?}) rotation {}]",
            self.original_index,
            self.name,
            self.rotation.value()
        )
    }
}

/// Used to identify a specific variation of an input model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelInstance {
    /// Index of the [`Model`] this was expanded from
    pub model_index: ModelIndex,
    /// Rotation of the [`Model`]
    pub rotation: ModelRotation,
}

/// Represents a rotation around an Axis, in the trigonometric(counterclockwise) direction
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ModelRotation {
    /// Rotation of 0°
    Rot0,
    /// Rotation of 90°
    Rot90,
    /// Rotation of 180°
    Rot180,
    /// Rotation of 270°
    Rot270,
}

impl ModelRotation {
    /// Returns the value of the rotation in °(degrees).
    pub fn value(&self) -> u32 {
        match *self {
            ModelRotation::Rot0 => 0,
            ModelRotation::Rot90 => 90,
            ModelRotation::Rot180 => 180,
            ModelRotation::Rot270 => 270,
        }
    }
    /// Returns the index of the enum member in the enumeration.
    pub fn index(&self) -> u8 {
        match *self {
            ModelRotation::Rot0 => 0,
            ModelRotation::Rot90 => 1,
            ModelRotation::Rot180 => 2,
            ModelRotation::Rot270 => 3,
        }
    }

    #[inline]
    /// Returns a new [`ModelRotation`] equal to this rotation rotated by `rotation` counter-clock
    ///
    /// ### Example
    /// ```
    /// use ghx_proc_gen::generator::model::ModelRotation;
    ///
    /// let rot_90 = ModelRotation::Rot90;
    /// assert_eq!(rot_90.rotated(ModelRotation::Rot180), ModelRotation::Rot270);
    /// ```
    pub fn rotated(&self, rotation: ModelRotation) -> ModelRotation {
        ALL_NODE_ROTATIONS
            [(self.index() as usize + rotation.index() as usize) % ALL_NODE_ROTATIONS.len()]
    }

    #[inline]
    /// Modifies this rotation by rotating it by `rotation` counter-clock
    ///     
    /// ### Example
    /// ```
    /// use ghx_proc_gen::generator::model::ModelRotation;
    ///
    /// let mut rot = ModelRotation::Rot90;
    /// rot.rotate(ModelRotation::Rot180);
    /// assert_eq!(rot, ModelRotation::Rot270);
    /// ```
    pub fn rotate(&mut self, rotation: ModelRotation) {
        *self = self.rotated(rotation);
    }

    #[inline]
    /// Returns the next [`ModelRotation`]: this rotation rotated by 90° counter-clockwise.
    ///
    /// ### Example
    /// ```
    /// use ghx_proc_gen::generator::model::ModelRotation;
    ///
    /// let rot_90 = ModelRotation::Rot90;
    /// let rot_180 = rot_90.next();
    /// assert_eq!(rot_180, ModelRotation::Rot180);
    /// ```
    pub fn next(&self) -> ModelRotation {
        self.rotated(ModelRotation::Rot90)
    }
}

/// All the possible rotations for a [`Model`]
pub const ALL_NODE_ROTATIONS: &'static [ModelRotation] = &[
    ModelRotation::Rot0,
    ModelRotation::Rot90,
    ModelRotation::Rot180,
    ModelRotation::Rot270,
];

pub(crate) fn expand_models<T: CoordinateSystem>(
    models: Vec<Model<T>>,
    rotation_axis: Direction,
) -> Vec<ExpandedModel> {
    let mut expanded_models = Vec::new();
    for (index, model) in models.iter().enumerate() {
        // Iterate on a vec of all possible node rotations and filter with the set to have a deterministic insertion order of expanded nodes.
        for rotation in ALL_NODE_ROTATIONS {
            if model.allowed_rotations.contains(&rotation) {
                let rotated_sockets = model.rotated_sockets(*rotation, rotation_axis);
                expanded_models.push(ExpandedModel {
                    sockets: rotated_sockets
                        .iter()
                        .map(|dir| dir.iter().map(|s| s.id()).collect())
                        .collect(),
                    weight: model.weight,
                    original_index: index,
                    rotation: *rotation,
                    #[cfg(feature = "debug-traces")]
                    name: model.name,
                });
            }
        }
    }
    expanded_models
}
