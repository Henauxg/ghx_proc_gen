use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt,
    marker::PhantomData,
};

use ghx_grid::{
    coordinate_system::{Cartesian2D, Cartesian3D, CoordinateSystem},
    direction::{Direction, DirectionTrait},
};
use ndarray::{Array, Ix1, Ix2};

#[cfg(feature = "models-names")]
use std::borrow::Cow;

#[cfg(feature = "debug-traces")]
use tracing::trace;

#[cfg(feature = "bevy")]
use bevy::ecs::component::Component;
#[cfg(feature = "reflect")]
use bevy::{ecs::reflect::ReflectComponent, reflect::Reflect};

use super::{
    model::{
        Model, ModelCollection, ModelIndex, ModelInstance, ModelRotation, ModelVariantIndex,
        ALL_MODEL_ROTATIONS,
    },
    socket::SocketCollection,
};
use crate::{NodeSetError, RulesBuilderError};

/// Rotation axis in a 2D cartesian coordinate system
pub const CARTESIAN_2D_ROTATION_AXIS: Direction = Direction::ZForward;

/// Used to create new [`Rules`]
pub struct RulesBuilder<C: CoordinateSystem> {
    models: ModelCollection<C>,
    socket_collection: SocketCollection,
    rotation_axis: C::Direction,
    coord_system: C,
}

impl RulesBuilder<Cartesian2D> {
    /// Used to create Rules for a 2d cartesian grid.
    ///
    /// ### Example
    ///
    /// Create simple `Rules` for a chess-like pattern
    /// ```
    /// use ghx_proc_gen::{generator::{socket::{SocketsCartesian2D, SocketCollection}, rules::{Rules, RulesBuilder}, model::ModelCollection}};
    /// use ghx_grid::coordinate_system::Cartesian2D;
    ///
    /// let mut sockets = SocketCollection::new();
    /// let (white, black) = (sockets.create(), sockets.create());
    /// sockets.add_connection(white, vec![black]);
    ///
    /// let mut models = ModelCollection::<Cartesian2D>::new();
    /// models.create(SocketsCartesian2D::Mono(white));
    /// models.create(SocketsCartesian2D::Mono(black));
    ///
    /// let rules = RulesBuilder::new_cartesian_2d(models, sockets).build().unwrap();
    /// ```
    pub fn new_cartesian_2d(
        models: ModelCollection<Cartesian2D>,
        socket_collection: SocketCollection,
    ) -> Self {
        Self {
            models,
            socket_collection,
            rotation_axis: CARTESIAN_2D_ROTATION_AXIS,
            coord_system: Cartesian2D,
        }
    }
}
impl RulesBuilder<Cartesian3D> {
    /// Used to create Rules for a 3d cartesian grid.
    ///
    /// ### Example
    ///
    /// Create simple `Rules` to describe an empty room with variable length pillars (with Y up in a right-handed coordinate system).
    /// ```
    /// use ghx_grid::{grid::GridDefinition, coordinate_system::Cartesian2D};
    /// use ghx_proc_gen::generator::{socket::{SocketsCartesian3D, SocketCollection}, rules::{Rules, RulesBuilder}, model::ModelCollection};
    ///
    /// let mut sockets = SocketCollection::new();
    /// let void = sockets.create();
    /// let (pillar_base_top, pillar_core_bottom, pillar_core_top, pillar_cap_bottom) = (sockets.create(), sockets.create(), sockets.create(), sockets.create());
    ///
    /// let mut models = ModelCollection::new();
    /// models.create(SocketsCartesian3D::Mono(void));
    /// models.create(SocketsCartesian3D::Simple {
    ///         x_pos: void,
    ///         x_neg: void,
    ///         z_pos: void,
    ///         z_neg: void,
    ///         y_pos: pillar_base_top,
    ///         y_neg: void,
    /// });
    /// models.create(SocketsCartesian3D::Simple {
    ///         x_pos: void,
    ///         x_neg: void,
    ///         z_pos: void,
    ///         z_neg: void,
    ///         y_pos: pillar_core_top,
    ///         y_neg: pillar_core_bottom,
    /// });
    /// models.create(SocketsCartesian3D::Simple {
    ///         x_pos: void,
    ///         x_neg: void,
    ///         z_pos: void,
    ///         z_neg: void,
    ///         y_pos: void,
    ///         y_neg: pillar_cap_bottom,
    /// });
    ///
    /// sockets.add_connections(vec![
    ///     (void, vec![void]),
    ///     (pillar_base_top, vec![pillar_core_bottom]),
    ///     (pillar_core_top, vec![pillar_core_bottom, pillar_cap_bottom]),
    /// ]);
    /// let rules = RulesBuilder::new_cartesian_3d(models, sockets).build().unwrap();
    /// ```
    pub fn new_cartesian_3d(
        models: ModelCollection<Cartesian3D>,
        socket_collection: SocketCollection,
    ) -> Self {
        Self {
            models,
            socket_collection,
            rotation_axis: Direction::YForward,
            coord_system: Cartesian3D,
        }
    }
}

impl RulesBuilder<Cartesian3D> {
    /// Sets the [`Direction`] to be used in the [`Rules`] as the rotation axis for the models
    pub fn with_rotation_axis(mut self, rotation_axis: Direction) -> RulesBuilder<Cartesian3D> {
        self.rotation_axis = rotation_axis;
        self
    }
}

impl<C: CoordinateSystem> RulesBuilder<C> {
    /// Builds the [`Rules`] from the current configuration of the [`RulesBuilder`]
    ///
    /// May return [`crate::RulesBuilderError::NoModelsOrSockets`] if `models` or `socket_collection` are empty.
    pub fn build(self) -> Result<Rules<C>, RulesBuilderError> {
        Rules::new(
            self.models,
            self.socket_collection,
            self.rotation_axis,
            self.coord_system,
        )
    }
}

/// Information about a Model
#[derive(Clone, Debug)]
#[cfg_attr(feature = "bevy", derive(Component, Default))]
#[cfg_attr(feature = "reflect", derive(Reflect), reflect(Component))]
pub struct ModelInfo {
    /// Weight of the original [`Model`]
    pub weight: f32,

    /// Name given to the original [`Model`]
    #[cfg(feature = "models-names")]
    pub name: Cow<'static, str>,
}

impl fmt::Display for ModelInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "w: {}", self.weight)?;
        #[cfg(feature = "models-names")]
        write!(f, ", {}", self.name)
    }
}

/// Defines the rules of a generation: the coordinate system, the models, the way they can be rotated, the sockets and their connections.
///
/// A same set of [`Rules`] can be shared by multiple generators.
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Rules<C: CoordinateSystem> {
    /// Number of original input models used to build these rules.
    original_models_count: usize,
    /// Maps a [`super::model::ModelIndex`] and a [`super::model::ModelRotation`] to an optionnal corresponding [`ModelVariantIndex`]
    models_mapping: Array<Option<ModelVariantIndex>, Ix2>,

    /// All the model variations in this ruleset.
    ///
    /// This is expanded from a given collection of base models, with added variations of rotations around an axis.
    models: Vec<ModelInstance>,
    weights: Vec<f32>,
    #[cfg(feature = "models-names")]
    names: Vec<Option<Cow<'static, str>>>,

    /// The vector `allowed_neighbours[model_index][direction]` holds all the allowed adjacent models (indexes) to `model_index` in `direction`.
    ///
    /// Calculated from models variations.
    ///
    /// Note: this cannot be a simple 3d array since the third dimension is different for each element.
    allowed_neighbours: Array<Vec<usize>, Ix2>,

    typestate: PhantomData<C>,
}

impl<C: CoordinateSystem> Rules<C> {
    fn new(
        models: ModelCollection<C>,
        socket_collection: SocketCollection,
        rotation_axis: C::Direction,
        coord_system: C,
    ) -> Result<Rules<C>, RulesBuilderError> {
        let original_models_count = models.models_count();
        let mut model_variations = models.create_variations(rotation_axis);
        // We test the expanded models because a model may have no rotations allowed.
        if model_variations.len() == 0 || socket_collection.is_empty() {
            return Err(RulesBuilderError::NoModelsOrSockets);
        }

        // Temporary collection to reverse the relation: sockets_to_models.get(socket)[direction] will hold all the models that have 'socket' from 'direction'
        let mut sockets_to_models = HashMap::new();
        // Using a BTreeSet because HashSet order is not deterministic. Performance impact is non-existant since `sockets_to_models` is discarded after building the Rules.
        let empty_in_all_directions: Array<BTreeSet<ModelVariantIndex>, Ix1> =
            Array::from_elem(coord_system.directions().len(), BTreeSet::new());
        for (model_index, model) in model_variations.iter().enumerate() {
            for &direction in coord_system.directions() {
                let opposite_dir: usize = direction.opposite().into();
                for socket in &model.sockets()[direction.into()] {
                    let compatible_models = sockets_to_models
                        .entry(socket)
                        .or_insert(empty_in_all_directions.clone());
                    compatible_models[opposite_dir].insert(model_index);
                }
            }
        }

        let mut allowed_neighbours = Array::from_elem(
            (model_variations.len(), coord_system.directions().len()),
            Vec::new(),
        );
        for (model_index, model) in model_variations.iter().enumerate() {
            for &direction in coord_system.directions() {
                // We filter unique models with a Set, but waht we want in the Rules is a Vec for access speed, caching, and iteration determinism.
                let mut unique_models = HashSet::new();
                // For each socket of the model in this direction: get all the sockets that are compatible for connection
                for socket in &model.sockets()[direction.into()] {
                    if let Some(compatible_sockets) = socket_collection.get_compatibles(*socket) {
                        for compatible_socket in compatible_sockets {
                            // For each of those: get all the models that have this socket from direction
                            // `sockets_to_models` may not have an entry for `compatible_socket` depending on user input data (socket present in sockets_connections but not in a model)
                            if let Some(allowed_models) = sockets_to_models.get(&compatible_socket)
                            {
                                for allowed_model in &allowed_models[direction.into()] {
                                    if unique_models.insert(*allowed_model) {
                                        allowed_neighbours[(model_index, direction.into())]
                                            .push(*allowed_model);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Discard socket information, build linear buffers containing the info needed during the generation
        let mut weights = Vec::with_capacity(model_variations.len());
        let mut model_instances = Vec::with_capacity(model_variations.len());
        #[cfg(feature = "models-names")]
        let mut names = Vec::with_capacity(model_variations.len());

        let mut models_mapping =
            Array::from_elem((original_models_count, ALL_MODEL_ROTATIONS.len()), None);
        for (index, model_variation) in model_variations.iter_mut().enumerate() {
            weights.push(model_variation.weight());
            model_instances.push(model_variation.to_instance());
            #[cfg(feature = "models-names")]
            names.push(model_variation.name.take());

            models_mapping[(
                model_variation.original_index(),
                model_variation.rotation().index() as usize,
            )] = Some(index);
        }

        #[cfg(feature = "debug-traces")]
        {
            trace!(
                "Successfully built rules, allowed_neighbours: {:?}",
                allowed_neighbours
            );
        }

        Ok(Rules {
            original_models_count,
            models_mapping,
            models: model_instances,
            weights,
            #[cfg(feature = "models-names")]
            names,
            allowed_neighbours,
            typestate: PhantomData,
        })
    }

    #[inline]
    pub(crate) fn allowed_models<Direction: Into<usize>>(
        &self,
        model: ModelVariantIndex,
        direction: Direction,
    ) -> &Vec<ModelVariantIndex> {
        &self.allowed_neighbours[(model, direction.into())]
    }

    /// Returns the number of models (expanded from the input models) present in the rules
    #[inline]
    pub fn models_count(&self) -> usize {
        self.models.len()
    }

    /// Returns the number of original input models that were used to build these rules
    #[inline]
    pub fn original_models_count(&self) -> usize {
        self.original_models_count
    }

    #[inline]
    pub(crate) fn model(&self, index: ModelVariantIndex) -> &ModelInstance {
        &self.models[index]
    }

    pub(crate) fn model_info(&self, model_index: ModelVariantIndex) -> ModelInfo {
        ModelInfo {
            weight: self.weights[model_index],

            #[cfg(feature = "models-names")]
            name: self.name_unchecked(model_index),
        }
    }

    #[inline]
    pub(crate) fn weight_unchecked(&self, model_index: ModelVariantIndex) -> f32 {
        self.weights[model_index]
    }

    /// Returns the weight of a model variant as an [`Option`]. Returns [`None`] if this model variant index is not valid.
    pub fn weight(&self, model_index: ModelVariantIndex) -> Option<f32> {
        match self.is_valid_model_variant_index(model_index) {
            true => Some(self.weights[model_index]),
            false => None,
        }
    }

    #[inline]
    fn is_valid_model_variant_index(&self, model_index: ModelVariantIndex) -> bool {
        model_index < self.models.len()
    }

    /// Returns `Some` [`ModelVariantIndex`] corresponding to the original model with index `model_index` rotated by `rot`. Returns [`None`] if this variation does not exist.
    pub fn variant_index(
        &self,
        model_index: ModelIndex,
        rot: ModelRotation,
    ) -> Option<ModelVariantIndex> {
        if model_index < self.original_models_count {
            self.models_mapping[(model_index, rot.index() as usize)]
        } else {
            None
        }
    }

    #[cfg(feature = "models-names")]
    #[inline]
    pub(crate) fn name_unchecked(&self, model_index: ModelVariantIndex) -> Cow<'static, str> {
        match self.names[model_index].as_ref() {
            None => "None".into(),
            Some(name) => name.clone(),
        }
    }

    #[cfg(feature = "models-names")]
    #[inline]
    pub(crate) fn name_unchecked_str(&self, model_index: ModelVariantIndex) -> &str {
        match self.names[model_index].as_ref() {
            None => "None",
            Some(name) => name,
        }
    }

    /// Returns the name of a model variant as an [`Option`].
    ///
    /// Returns [`None`]  if this model variant index is not valid or if it does not have a name.
    #[cfg(feature = "models-names")]
    pub fn name_str(&self, model_index: ModelVariantIndex) -> Option<&str> {
        match self.is_valid_model_variant_index(model_index) {
            true => Some(self.name_unchecked_str(model_index)),
            false => None,
        }
    }
}

/// Represents a reference to a [`super::model::ModelVariation`] of some [`Rules`]
pub trait ModelVariantRef<C: CoordinateSystem> {
    /// Returns the [`ModelVariantIndex`] that is referenced by this `ModelVariantRef`.
    ///
    /// Returns a [`NodeSetError::InvalidModelRef`] if the reference is invalid in the [`Rules`].
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError>;
}
impl<C: CoordinateSystem> ModelVariantRef<C> for ModelVariantIndex {
    fn to_index(&self, _rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        Ok(*self)
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for Model<C> {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        let rot = self.first_rot();
        rules
            .variant_index(self.index(), rot)
            .ok_or(NodeSetError::InvalidModelRef(self.index(), rot))
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for &Model<C> {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        let rot = self.first_rot();
        rules
            .variant_index(self.index(), rot)
            .ok_or(NodeSetError::InvalidModelRef(self.index(), rot))
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for (ModelIndex, ModelRotation) {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        rules
            .variant_index(self.0, self.1)
            .ok_or(NodeSetError::InvalidModelRef(self.0, self.1))
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for (Model<C>, ModelRotation) {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        rules
            .variant_index(self.0.index(), self.1)
            .ok_or(NodeSetError::InvalidModelRef(self.0.index(), self.1))
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for (&Model<C>, ModelRotation) {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        rules
            .variant_index(self.0.index(), self.1)
            .ok_or(NodeSetError::InvalidModelRef(self.0.index(), self.1))
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for ModelInstance {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        rules
            .variant_index(self.model_index, self.rotation)
            .ok_or(NodeSetError::InvalidModelRef(
                self.model_index,
                self.rotation,
            ))
    }
}
impl<C: CoordinateSystem> ModelVariantRef<C> for &ModelInstance {
    fn to_index(&self, rules: &Rules<C>) -> Result<ModelVariantIndex, NodeSetError> {
        rules
            .variant_index(self.model_index, self.rotation)
            .ok_or(NodeSetError::InvalidModelRef(
                self.model_index,
                self.rotation,
            ))
    }
}
