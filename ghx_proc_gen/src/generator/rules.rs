use std::{
    collections::{BTreeSet, HashMap, HashSet},
    marker::PhantomData,
};

use ndarray::{Array, Ix1, Ix2};

#[cfg(feature = "debug-traces")]
use tracing::trace;

use super::{
    model::{expand_models, ExpandedModel, Model, ModelIndex},
    socket::SocketCollection,
};
use crate::{
    grid::direction::{Cartesian2D, Cartesian3D, CoordinateSystem, Direction},
    RulesError,
};

/// Rotation axis in a 2D cartesian coordinate system
pub const CARTESIAN_2D_ROTATION_AXIS: Direction = Direction::ZForward;

/// Used to create new [`Rules`]
pub struct RulesBuilder<T: CoordinateSystem + Clone> {
    models: Vec<Model<T>>,
    socket_collection: SocketCollection,
    rotation_axis: Direction,
    coord_system: T,
}

impl RulesBuilder<Cartesian2D> {
    /// Used to create Rules for a 2d cartesian grid.
    ///
    /// ### Example
    ///
    /// Create simple `Rules` for a chess-like pattern
    /// ```
    /// use ghx_proc_gen::generator::{socket::{SocketsCartesian2D, SocketCollection}, rules::{Rules, RulesBuilder}};
    ///
    /// let mut sockets = SocketCollection::new();
    /// let (white, black) = (sockets.create(), sockets.create());
    /// sockets.add_connection(white, vec![black]);
    /// let models = vec![
    ///     SocketsCartesian2D::Mono(white).new_model(),
    ///     SocketsCartesian2D::Mono(black).new_model(),
    /// ];
    /// let rules = RulesBuilder::new_cartesian_2d(models, sockets).build().unwrap();
    /// ```
    pub fn new_cartesian_2d(
        models: Vec<Model<Cartesian2D>>,
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
    /// Create simple `Rules` to describe an empty room with variable length pillars (with Y up in a right-handed cooridnate system).
    /// ```
    /// use ghx_proc_gen::grid::GridDefinition;
    /// use ghx_proc_gen::generator::{socket::{SocketsCartesian3D, SocketCollection}, rules::{Rules, RulesBuilder}};
    ///
    /// let mut sockets = SocketCollection::new();
    /// let void = sockets.create();
    /// let (pillar_base_top, pillar_core_bottom, pillar_core_top, pillar_cap_bottom) = (sockets.create(), sockets.create(), sockets.create(), sockets.create());
    /// let models = vec![
    ///     SocketsCartesian3D::Mono(void).new_model(),
    ///     SocketsCartesian3D::Simple {
    ///         x_pos: void,
    ///         x_neg: void,
    ///         z_pos: void,
    ///         z_neg: void,
    ///         y_pos: pillar_base_top,
    ///         y_neg: void,
    ///     }.new_model(),
    ///     SocketsCartesian3D::Simple {
    ///         x_pos: void,
    ///         x_neg: void,
    ///         z_pos: void,
    ///         z_neg: void,
    ///         y_pos: pillar_core_top,
    ///         y_neg: pillar_core_bottom,
    ///     }.new_model(),
    ///     SocketsCartesian3D::Simple {
    ///         x_pos: void,
    ///         x_neg: void,
    ///         z_pos: void,
    ///         z_neg: void,
    ///         y_pos: void,
    ///         y_neg: pillar_cap_bottom,
    ///     }.new_model(),
    /// ];
    /// sockets.add_connections(vec![
    ///     (void, vec![void]),
    ///     (pillar_base_top, vec![pillar_core_bottom]),
    ///     (pillar_core_top, vec![pillar_core_bottom, pillar_cap_bottom]),
    /// ]);
    /// let rules = RulesBuilder::new_cartesian_3d(models, sockets).build().unwrap();
    /// ```
    pub fn new_cartesian_3d(
        models: Vec<Model<Cartesian3D>>,
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

impl<T: CoordinateSystem + Clone> RulesBuilder<T> {
    /// Builds the [`Rules`] from the current configuration of the [`RulesBuilder`]
    ///
    /// May return [`crate::RulesError::NoModelsOrSockets`] if `models` or `socket_collection` are empty.
    pub fn build(self) -> Result<Rules<T>, RulesError> {
        Rules::new(
            self.models,
            self.socket_collection,
            self.rotation_axis,
            self.coord_system,
        )
    }
}

/// Defines the rules of a generation: the coordinate system, the models, the way they can be rotated, the sockets and their connections.
///
/// A same set of [`Rules`] can be shared by multiple generators.
pub struct Rules<T: CoordinateSystem> {
    /// Number of original input models used to build these rules.
    original_models_count: usize,
    /// All the models in this ruleset.
    ///
    /// This is expanded from a given collection of base models, with added variations of rotations around an axis.
    models: Vec<ExpandedModel>,
    /// The vector `allowed_neighbours[model_index][direction]` holds all the allowed adjacent models (indexes) to `model_index` in `direction`.
    ///
    /// Calculated from expanded models.
    ///
    /// Note: this cannot be a simple 3d array since the third dimension is different for each element.
    allowed_neighbours: Array<Vec<usize>, Ix2>,
    typestate: PhantomData<T>,
}

impl<T: CoordinateSystem> Rules<T> {
    fn new(
        models: Vec<Model<T>>,
        socket_collection: SocketCollection,
        rotation_axis: Direction,
        coord_system: T,
    ) -> Result<Rules<T>, RulesError> {
        let original_models_count = models.len();
        let expanded_models = expand_models(models, rotation_axis);
        // We test the expanded models because a model may have no rotations allowed.
        if expanded_models.len() == 0 || socket_collection.is_empty() {
            return Err(RulesError::NoModelsOrSockets);
        }

        // Temporary collection to reverse the relation: sockets_to_models.get(socket)[direction] will hold all the models that have 'socket' from 'direction'
        let mut sockets_to_models = HashMap::new();
        // Using a BTreeSet because HashSet order is not deterministic. Performance impact is non-existant since `sockets_to_models` is discarded after building the Rules.
        let empty_in_all_directions: Array<BTreeSet<ModelIndex>, Ix1> =
            Array::from_elem(coord_system.directions().len(), BTreeSet::new());
        for (model_index, model) in expanded_models.iter().enumerate() {
            for &direction in coord_system.directions() {
                let opposite_dir = direction.opposite() as usize;
                for socket in &model.sockets()[direction as usize] {
                    let compatible_models = sockets_to_models
                        .entry(socket)
                        .or_insert(empty_in_all_directions.clone());
                    compatible_models[opposite_dir].insert(model_index);
                }
            }
        }

        let mut allowed_neighbours = Array::from_elem(
            (expanded_models.len(), coord_system.directions().len()),
            Vec::new(),
        );
        for (model_index, model) in expanded_models.iter().enumerate() {
            for &direction in coord_system.directions() {
                // We filter unique models with a Set, but waht we want in the Rules is a Vec for access speed, caching, and iteration determinism.
                let mut unique_models = HashSet::new();
                // For each socket of the model in this direction: get all the sockets that are compatible for connection
                for socket in &model.sockets()[direction as usize] {
                    if let Some(compatible_sockets) = socket_collection.get_compatibles(*socket) {
                        for compatible_socket in compatible_sockets {
                            // For each of those: get all the models that have this socket from direction
                            // `sockets_to_models` may not have an entry for `compatible_socket` depending on user input data (socket present in sockets_connections but not in a model)
                            if let Some(allowed_models) = sockets_to_models.get(&compatible_socket)
                            {
                                for allowed_model in &allowed_models[direction as usize] {
                                    if unique_models.insert(*allowed_model) {
                                        allowed_neighbours[(model_index, direction as usize)]
                                            .push(*allowed_model);
                                    }
                                }
                            }
                        }
                    }
                }
            }
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
            models: expanded_models,
            allowed_neighbours,
            typestate: PhantomData,
        })
    }

    #[inline]
    pub(crate) fn allowed_models(
        &self,
        model: ModelIndex,
        direction: Direction,
    ) -> &Vec<ModelIndex> {
        &self.allowed_neighbours[(model, direction as usize)]
    }

    #[inline]
    pub(crate) fn weight(&self, model_index: ModelIndex) -> f32 {
        self.models[model_index].weight()
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
    pub(crate) fn model(&self, index: usize) -> &ExpandedModel {
        &self.models[index]
    }
}
