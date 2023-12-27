use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use ndarray::{Array, Ix1, Ix2};

#[cfg(feature = "debug-traces")]
use tracing::trace;

use super::node::{
    expand_models, ExpandedNodeModel, ModelIndex, NodeModel, Socket, SocketCollection,
};
use crate::{
    grid::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet},
    RulesError,
};

pub const CARTESIAN_2D_ROTATION_AXIS: Direction = Direction::ZForward;

pub type SocketConnections = (Socket, Vec<Socket>);

pub struct RulesBuilder<T: DirectionSet + Clone> {
    models: Vec<NodeModel<T>>,
    socket_collection: SocketCollection,
    rotation_axis: Direction,
    direction_set: T,
}

impl RulesBuilder<Cartesian2D> {
    /// Used to create Rules for a 2d cartesian grid.
    ///
    /// Will only return [`ProcGenError::InvalidRules`] if `models` or `sockets_connections` are empty.
    ///
    /// For `sockets_connections`, there is no need to specify a connection in both directions: `[0, vec![1]]` means that socket `0` can be connected to a socket `1`, so `[1, vec![0]]` is implied.
    ///
    /// ### Example
    ///
    /// Create simple `Rules` for a chess-like pattern
    /// ```
    /// use ghx_proc_gen::generator::{node::SocketsCartesian2D, rules::Rules};
    ///
    /// const WHITE: u32 = 0;
    /// const BLACK: u32 = 1;
    /// let models = vec![
    ///     SocketsCartesian2D::Mono(WHITE).new_model(),
    ///     SocketsCartesian2D::Mono(BLACK).new_model(),
    /// ];
    /// let sockets_connections = vec![(WHITE, vec![BLACK])];
    /// let rules = Rules::new_cartesian_2d(models, sockets_connections).unwrap();
    /// ```
    pub fn new_cartesian_2d(
        models: Vec<NodeModel<Cartesian2D>>,
        socket_collection: SocketCollection,
    ) -> Self {
        Self {
            models,
            socket_collection,
            rotation_axis: CARTESIAN_2D_ROTATION_AXIS,
            direction_set: Cartesian2D {},
        }
    }
}
impl RulesBuilder<Cartesian3D> {
    /// Used to create Rules for a 3d cartesian grid.
    ///
    /// Will only return [`ProcGenError::InvalidRules`] if `models` or `sockets_connections` are empty.
    ///
    /// For `sockets_connections`, there is no need to specify a connection in both directions: `[0, vec![1]]` means that socket `0` can be connected to a socket `1`, so `[1, vec![0]]` is implied.
    ///
    /// ### Example
    ///
    /// Create simple `Rules` to describe an empty room with variable length pillars (with Y up in a right-handed cooridnate system).
    /// ```
    /// use ghx_proc_gen::grid::GridDefinition;
    /// use ghx_proc_gen::generator::{node::{SocketsCartesian3D, SocketId}, rules::Rules};
    ///
    /// const VOID: SocketId = 0;
    /// const PILLAR_BASE_TOP: SocketId = 1;
    /// const PILLAR_CORE_BOTTOM: SocketId = 2;
    /// const PILLAR_CORE_TOP: SocketId = 3;
    /// const PILLAR_CAP_BOTTOM: SocketId = 4;
    ///
    /// let models = vec![
    ///     SocketsCartesian3D::Mono(VOID).new_model(),
    ///     SocketsCartesian3D::Simple(VOID, PILLAR_BASE_TOP, VOID, VOID, VOID, VOID).new_model(),
    ///     SocketsCartesian3D::Simple(VOID, PILLAR_CORE_TOP, VOID, PILLAR_CORE_BOTTOM, VOID, VOID).new_model(),
    ///     SocketsCartesian3D::Simple(VOID, VOID, VOID, PILLAR_CAP_BOTTOM, VOID, VOID).new_model(),
    /// ];
    /// let sockets_connections = vec![
    ///     (VOID, vec![VOID]),
    ///     (PILLAR_BASE_TOP, vec![PILLAR_CORE_BOTTOM]),
    ///     (PILLAR_CORE_TOP, vec![PILLAR_CORE_BOTTOM, PILLAR_CAP_BOTTOM]),
    /// ];
    /// let rules = Rules::new_cartesian_3d(models, sockets_connections).unwrap();
    /// ```
    pub fn new_cartesian_3d(
        models: Vec<NodeModel<Cartesian3D>>,
        socket_collection: SocketCollection,
    ) -> Self {
        Self {
            models,
            socket_collection,
            rotation_axis: Direction::YForward,
            direction_set: Cartesian3D {},
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

impl<T: DirectionSet + Clone> RulesBuilder<T> {
    pub fn build(self) -> Result<Rules<T>, RulesError> {
        Rules::new(
            self.models,
            self.socket_collection,
            self.rotation_axis,
            self.direction_set,
        )
    }
}

pub struct Rules<T: DirectionSet> {
    /// All the models in this ruleset.
    ///
    /// Expanded from a given set of base models with added variations of rotations around an axis.
    models: Vec<ExpandedNodeModel>,
    /// The vector `allowed_neighbours[model_index][direction]` holds all the allowed adjacent models (indexes) to `model_index` in `direction`.
    ///
    /// Calculated from expanded models.
    ///
    /// Note: this cannot be a simple 3d array since the third dimension is different for each element.
    allowed_neighbours: Array<Vec<usize>, Ix2>,
    typestate: PhantomData<T>,
}

impl<T: DirectionSet> Rules<T> {
    fn new(
        models: Vec<NodeModel<T>>,
        socket_collection: SocketCollection,
        rotation_axis: Direction,
        direction_set: T,
    ) -> Result<Rules<T>, RulesError> {
        if models.len() == 0 || socket_collection.is_empty() {
            return Err(RulesError::NoModelsOrSockets);
        }

        let expanded_models = expand_models(models, rotation_axis);

        // Temporary collection to reverse the relation: sockets_to_models.get(socket)[direction] will hold all the models that have 'socket' from 'direction'
        let mut sockets_to_models = HashMap::new();
        let empty_in_all_directions: Array<HashSet<ModelIndex>, Ix1> =
            Array::from_elem(direction_set.directions().len(), HashSet::new());
        for (model_index, model) in expanded_models.iter().enumerate() {
            for &direction in direction_set.directions() {
                let inverse_dir = direction.opposite() as usize;
                for socket in &model.sockets()[direction as usize] {
                    let compatible_models = sockets_to_models
                        .entry(socket)
                        .or_insert(empty_in_all_directions.clone());
                    compatible_models[inverse_dir].insert(model_index);
                }
            }
        }

        let mut allowed_neighbours = Array::from_elem(
            (expanded_models.len(), direction_set.directions().len()),
            Vec::new(),
        );
        for (model_index, model) in expanded_models.iter().enumerate() {
            for &direction in direction_set.directions() {
                let mut unique_models = HashSet::new();
                // For each socket of the model in this direction: get all the sockets that are compatible for connection
                for socket in &model.sockets()[direction as usize] {
                    if let Some(compatible_sockets) = socket_collection.get_compatibles(*socket) {
                        for compatible_socket in compatible_sockets {
                            // For each of those: get all the models that have this socket from direction
                            // `sockets_to_models` may not have an entry for `compatible_socket` depending on user input data (socket present in sockets_connections but not in a model)
                            if let Some(entry) = sockets_to_models.get(&compatible_socket) {
                                for allowed_model in &entry[direction as usize] {
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

    /// Returns the count of models (expanded from the input models) present in the rules
    #[inline]
    pub fn models_count(&self) -> usize {
        self.models.len()
    }

    #[inline]
    pub(crate) fn model(&self, index: usize) -> &ExpandedNodeModel {
        &self.models[index]
    }
}
