use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use ndarray::{Array, Ix1, Ix2};

use crate::{
    grid::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet},
    ProcGenError,
};

use super::node::{expand_models, ExpandedNodeModel, ModelIndex, NodeModel, SocketId};

pub type SocketConnections = (SocketId, Vec<SocketId>);

pub struct Rules<T: DirectionSet> {
    /// All the models in this ruleset. Expanded from a given set of a base models with generated variations of rotations around the Z axis.
    models: Vec<ExpandedNodeModel>,
    /// The vector `allowed_neighbours[model_index][direction]` holds all the allowed adjacent models (indexes) to `model_index` in `direction`.
    ///
    /// Calculated from expanded models.
    ///
    /// Note: this cannot be a simple 3d array since the third dimension is different for each element.
    allowed_neighbours: Array<Vec<usize>, Ix2>,
    typestate: PhantomData<T>,
}

impl Rules<Cartesian2D> {
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
        sockets_connections: Vec<SocketConnections>,
    ) -> Result<Rules<Cartesian2D>, ProcGenError> {
        Self::new(models, sockets_connections, Cartesian2D {})
    }
}

impl Rules<Cartesian3D> {
    /// Used to create Rules for a 3d cartesian grid.
    ///
    /// Will only return [`ProcGenError::InvalidRules`] if `models` or `sockets_connections` are empty.
    ///
    /// For `sockets_connections`, there is no need to specify a connection in both directions: `[0, vec![1]]` means that socket `0` can be connected to a socket `1`, so `[1, vec![0]]` is implied.
    ///
    /// ### Example
    ///
    /// Create simple `Rules` to describe an empty room with variable length pillars.
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
    ///     SocketsCartesian3D::Simple(VOID, VOID, VOID, VOID, PILLAR_BASE_TOP, VOID).new_model(),
    ///     SocketsCartesian3D::Simple(VOID, VOID, VOID, VOID, PILLAR_CORE_TOP, PILLAR_CORE_BOTTOM)
    ///         .new_model(),
    ///     SocketsCartesian3D::Simple(VOID, VOID, VOID, VOID, VOID, PILLAR_CAP_BOTTOM).new_model(),
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
        sockets_connections: Vec<SocketConnections>,
    ) -> Result<Rules<Cartesian3D>, ProcGenError> {
        Self::new(models, sockets_connections, Cartesian3D {})
    }
}

impl<T: DirectionSet> Rules<T> {
    fn new(
        models: Vec<NodeModel<T>>,
        sockets_connections: Vec<SocketConnections>,
        direction_set: T,
    ) -> Result<Rules<T>, ProcGenError> {
        if models.len() == 0 || sockets_connections.len() == 0 {
            return Err(ProcGenError::InvalidRules);
        }

        let expanded_models = expand_models(models);
        let socket_to_sockets = expand_sockets_connections(sockets_connections);

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
                    if let Some(compatible_sockets) = socket_to_sockets.get(socket) {
                        for compatible_socket in compatible_sockets {
                            // For each of those: get all the models that have this socket from direction
                            // `sockets_to_models` may not have an entry for `compatible_socket` depending on user input data (socket present in sockets_connections but not in a model)
                            if let Some(entry) = sockets_to_models.get(compatible_socket) {
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

/// Expand sockets connections. Returns `socket_to_sockets`: from a socket, get all sockets that are compatible for connection
fn expand_sockets_connections(sockets_connections: Vec<(u32, Vec<u32>)>) -> HashMap<u32, Vec<u32>> {
    // 2 collections. One temporary to filter for uniqueness, one with a Vec for iteration determinism while generating later.
    let mut socket_to_sockets = HashMap::new();
    let mut unique_socket_to_sockets = HashMap::new();
    for (socket, connections) in sockets_connections {
        let connectable_sockets = unique_socket_to_sockets
            .entry(socket)
            .or_insert(HashSet::new());
        for other_socket in &connections {
            if connectable_sockets.insert(*other_socket) {
                socket_to_sockets
                    .entry(socket)
                    .or_insert(Vec::new())
                    .push(*other_socket);
            }
        }

        // Register the connection from the other socket too. (Doing it with a second iteration because unique_socket_to_sockets is already borrowed)
        for other_socket in &connections {
            if *other_socket != socket {
                let other_connectable_sockets = unique_socket_to_sockets
                    .entry(*other_socket)
                    .or_insert(HashSet::new());
                if other_connectable_sockets.insert(socket) {
                    socket_to_sockets
                        .entry(*other_socket)
                        .or_insert(Vec::new())
                        .push(socket);
                }
            }
        }
    }
    socket_to_sockets
}