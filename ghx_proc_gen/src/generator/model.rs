use std::{collections::HashSet, marker::PhantomData};

#[cfg(feature = "debug-traces")]
use {core::fmt, tracing::warn};

use crate::grid::direction::{Cartesian2D, Cartesian3D, CoordinateSystem, Direction};

use super::{
    rules::CARTESIAN_2D_ROTATION_AXIS,
    socket::{Socket, SocketId, SocketsCartesian2D, SocketsCartesian3D},
};

/// Index of an original model
pub type ModelIndex = usize;

/// Index of a model variation
pub type ModelVariantIndex = usize;

/// Default weight of [`Model`] and [`ModelTemplate`]
pub const DEFAULT_MODEL_WEIGHT: f32 = 1.0;

#[derive(Clone)]
/// Most of the information about a [`Model`] (but notably without any [`ModelIndex`]).
///
/// Can be used to create common shared templates before creating real models through a [`ModelCollection`]
pub struct ModelTemplate<C> {
    /// Allowed connections for this [`ModelTemplate`] in the output.
    sockets: Vec<Vec<Socket>>,
    /// Weight factor influencing the density of this [`ModelTemplate`] in the generated output.
    ///
    ///  Defaults to [`DEFAULT_MODEL_WEIGHT`]
    weight: f32,
    /// Allowed rotations of this [`ModelTemplate`] in the output, around the rotation axis specified in the rules.
    ///
    /// Defaults to only [`ModelRotation::Rot0`].
    ///
    /// Notes:
    /// - In 3d, sockets of a model that are on the rotation axis are rotated into new sockets when the model itself is rotated. See [`crate::generator::socket::SocketCollection`] for how to define and/or constrain sockets connections on the rotation axis.
    /// - In 2d, the rotation axis cannot be modified and is set to [`Direction::ZForward`].
    allowed_rotations: HashSet<ModelRotation>,
    typestate: PhantomData<C>,
}

impl ModelTemplate<Cartesian3D> {
    pub(crate) fn new(sockets: SocketsCartesian3D) -> ModelTemplate<Cartesian3D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([ModelRotation::Rot0]),
            weight: DEFAULT_MODEL_WEIGHT,
            typestate: PhantomData,
        }
    }

    /// Returns a clone of the [`Model`] with its sockets rotated by `rotation` around `axis`.
    pub fn rotated(&self, rotation: ModelRotation, axis: Direction) -> Self {
        Self {
            sockets: self.rotated_sockets(rotation, axis),
            weight: self.weight,
            allowed_rotations: self.allowed_rotations.clone(),
            typestate: PhantomData,
        }
    }
}

impl ModelTemplate<Cartesian2D> {
    pub(crate) fn new(sockets: SocketsCartesian2D) -> ModelTemplate<Cartesian2D> {
        Self {
            sockets: sockets.into(),
            allowed_rotations: HashSet::from([ModelRotation::Rot0]),
            weight: DEFAULT_MODEL_WEIGHT,
            typestate: PhantomData,
        }
    }

    /// Returns a clone of the [`Model`] with its sockets rotated by `rotation` around [`CARTESIAN_2D_ROTATION_AXIS`].
    pub fn rotated(&self, rotation: ModelRotation) -> Self {
        Self {
            sockets: self.rotated_sockets(rotation, CARTESIAN_2D_ROTATION_AXIS),
            weight: self.weight,
            allowed_rotations: self.allowed_rotations.clone(),
            typestate: PhantomData,
        }
    }
}

impl<C> ModelTemplate<C> {
    /// Specify that this [`ModelTemplate`] can be rotated in exactly one way: `rotation`
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotation(mut self, rotation: ModelRotation) -> Self {
        self.allowed_rotations = HashSet::from([rotation]);
        self
    }
    /// Specify that this [`ModelTemplate`] can be rotated by `rotation`, in addition to its currently allowed rotations.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_additional_rotation(mut self, rotation: ModelRotation) -> Self {
        self.allowed_rotations.insert(rotation);
        self
    }
    /// Specify that this [`ModelTemplate`] can be rotated in every way specified in `rotations`.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotations<R: Into<HashSet<ModelRotation>>>(mut self, rotations: R) -> Self {
        self.allowed_rotations = rotations.into();
        self
    }
    /// Specify that this [`ModelTemplate`] can be rotated in every way specified in `rotations` in addition to its currently allowed rotations.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_additional_rotations<R: IntoIterator<Item = ModelRotation>>(
        mut self,
        rotations: R,
    ) -> Self {
        self.allowed_rotations.extend(rotations.into_iter());
        self
    }
    /// Specify that this [`ModelTemplate`] can be rotated in every way.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_all_rotations(mut self) -> Self {
        self.allowed_rotations = ALL_MODEL_ROTATIONS.iter().cloned().collect();
        self
    }

    /// Specify this [`ModelTemplate`] weight. The `weight` value should be strictly superior to `0`. If it is not the case, the value will be overriden by `f32::MIN_POSITIVE`.
    ///
    /// Used by a [`super::Generator`] when using [`super::ModelSelectionHeuristic::WeightedProbability`] and [`super::node_heuristic::NodeSelectionHeuristic::MinimumEntropy`].
    ///
    /// All the variations (rotations) of this [`ModelTemplate`] will use the same weight.
    pub fn with_weight<W: Into<f32>>(mut self, weight: W) -> Self {
        let mut checked_weight = weight.into();
        if checked_weight <= 0. {
            #[cfg(feature = "debug-traces")]
            warn!(
                "Template had an invalid weight {} <= 0., weight overriden to f32::MIN: {}",
                checked_weight,
                f32::MIN_POSITIVE
            );
            checked_weight = f32::MIN_POSITIVE
        };
        self.weight = checked_weight;
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

/// Used to create one or more [`Model`]. Created models can then be used in a [`super::rules::RulesBuilder`]
pub struct ModelCollection<C: CoordinateSystem> {
    models: Vec<Model<C>>,
}

impl<C: CoordinateSystem> ModelCollection<C> {
    /// Creates a new [`ModelCollection`]
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    /// Creates a new [`Model`] in this collection and returns a reference to it.
    ///
    /// It can create a model from any type that can be turned into a [`ModelTemplate`]: sockets, a model template, or even another model.
    pub fn create<T: Into<ModelTemplate<C>>>(&mut self, template: T) -> &mut Model<C> {
        let model = Model::<C>::from_template(template.into(), self.models.len());
        self.models.push(model);
        self.models.last_mut().unwrap()
    }

    /// Returns how many [`Model`] are in this collection
    pub fn models_count(&self) -> usize {
        self.models.len()
    }

    pub(crate) fn create_variations(&self, rotation_axis: Direction) -> Vec<ModelVariation> {
        let mut model_variations = Vec::new();
        for model in self.models.iter() {
            // Iterate on a vec of all possible node rotations and filter with the set to have a deterministic insertion order of model variations.
            for rotation in ALL_MODEL_ROTATIONS {
                if model.template.allowed_rotations.contains(&rotation) {
                    let rotated_sockets = model.template.rotated_sockets(*rotation, rotation_axis);
                    model_variations.push(ModelVariation {
                        sockets: rotated_sockets
                            .iter()
                            .map(|dir| dir.iter().map(|s| s.id()).collect())
                            .collect(),
                        weight: model.template.weight,
                        original_index: model.index,
                        rotation: *rotation,
                        #[cfg(feature = "debug-traces")]
                        name: model.name,
                    });
                }
            }
        }
        model_variations
    }
}

/// Represents a model to be used by a [`crate::generator::Generator`] as a "building-block" to fill out the generated area.
#[derive(Clone)]
pub struct Model<C: CoordinateSystem> {
    index: ModelIndex,
    template: ModelTemplate<C>,

    /// Name given to this model for debug purposes.
    #[cfg(feature = "debug-traces")]
    name: Option<&'static str>,
}

impl<C: CoordinateSystem> Model<C> {
    pub(crate) fn from_template(template: ModelTemplate<C>, index: ModelIndex) -> Model<C> {
        Self {
            index,
            template,
            #[cfg(feature = "debug-traces")]
            name: None,
        }
    }

    /// Returns the [`ModelIndex`] of the model
    pub fn index(&self) -> ModelIndex {
        self.index
    }

    /// Specify that this [`Model`] can be rotated in exactly one way: `rotation`
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotation(&mut self, rotation: ModelRotation) -> &mut Self {
        self.template.allowed_rotations = HashSet::from([rotation]);
        self
    }
    /// Specify that this [`Model`] can be rotated by `rotation`, in addition to its currently allowed rotations.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_additional_rotation(&mut self, rotation: ModelRotation) -> &mut Self {
        self.template.allowed_rotations.insert(rotation);
        self
    }
    /// Specify that this [`Model`] can be rotated in every way specified in `rotations`.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_rotations<R: Into<HashSet<ModelRotation>>>(&mut self, rotations: R) -> &mut Self {
        self.template.allowed_rotations = rotations.into();
        self
    }
    /// Specify that this [`Model`] can be rotated in every way specified in `rotations` in addition to its currently allowed rotations.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_additional_rotations<R: IntoIterator<Item = ModelRotation>>(
        &mut self,
        rotations: R,
    ) -> &mut Self {
        self.template
            .allowed_rotations
            .extend(rotations.into_iter());
        self
    }
    /// Specify that this [`Model`] can be rotated in every way.
    ///
    /// Rotations are specified as counter-clockwise
    pub fn with_all_rotations(&mut self) -> &mut Self {
        self.template.allowed_rotations = ALL_MODEL_ROTATIONS.iter().cloned().collect();
        self
    }

    /// Specify this [`Model`] weight. The `weight` value should be strictly superior to `0`. If it is not the case, the value will be overriden by `f32::MIN_POSITIVE`.
    ///
    /// Used by a [`super::Generator`] when using [`super::ModelSelectionHeuristic::WeightedProbability`] and [`super::node_heuristic::NodeSelectionHeuristic::MinimumEntropy`].
    ///
    /// All the variations (rotations) of this [`Model`] will use the same weight.
    pub fn with_weight<W: Into<f32>>(&mut self, weight: W) -> &mut Self {
        let mut checked_weight = weight.into();
        if checked_weight <= 0. {
            #[cfg(feature = "debug-traces")]
            warn!(
                "Model with index {}, name {:?}, had an invalid weight {} <= 0., weight overriden to f32::MIN: {}",
                self.index, self.name, checked_weight, f32::MIN_POSITIVE
            );
            checked_weight = f32::MIN_POSITIVE
        };
        self.template.weight = checked_weight;
        self
    }

    #[allow(unused_mut)]
    /// Register the given name for this model.
    ///
    /// Does nothing if the `debug-traces` feature is not enabled.
    pub fn with_name(&mut self, _name: &'static str) -> &mut Self {
        #[cfg(feature = "debug-traces")]
        {
            self.name = Some(_name);
        }

        self
    }

    pub(crate) fn first_rot(&self) -> ModelRotation {
        for rot in ALL_MODEL_ROTATIONS {
            if self.template.allowed_rotations.contains(rot) {
                return *rot;
            }
        }
        ModelRotation::Rot0
    }
}
impl<C: CoordinateSystem> Into<ModelTemplate<C>> for Model<C> {
    fn into(self) -> ModelTemplate<C> {
        self.template.clone()
    }
}

/// This is a variation of a user [`Model`] generated by the [`crate::generator::Rules`]. One [`Model`] may be transformed into one ore more [`ModelVariation`] depending on the number of allowed rotations of the model.
#[derive(Debug)]
pub struct ModelVariation {
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

impl ModelVariation {
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

/// Used to identify a specific variation of an input model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelInstance {
    /// Index of the original [`Model`]
    pub model_index: ModelIndex,
    /// Rotation of the original [`Model`]
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
    /// Returns the value of the rotation in radians.
    pub fn rad(&self) -> f32 {
        f32::to_radians(self.value() as f32)
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
        ALL_MODEL_ROTATIONS
            [(self.index() as usize + rotation.index() as usize) % ALL_MODEL_ROTATIONS.len()]
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
pub const ALL_MODEL_ROTATIONS: &'static [ModelRotation] = &[
    ModelRotation::Rot0,
    ModelRotation::Rot90,
    ModelRotation::Rot180,
    ModelRotation::Rot270,
];
