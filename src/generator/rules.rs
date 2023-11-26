use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use ndarray::{Array, Ix1, Ix2};

use crate::grid::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet};

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
    pub fn new_cartesian_2d(
        models: Vec<NodeModel<Cartesian2D>>,
        sockets_connections: Vec<SocketConnections>,
    ) -> Rules<Cartesian2D> {
        Self::new(models, sockets_connections, Cartesian2D {})
    }
}

impl Rules<Cartesian3D> {
    pub fn new_cartesian_3d(
        models: Vec<NodeModel<Cartesian3D>>,
        sockets_connections: Vec<SocketConnections>,
    ) -> Rules<Cartesian3D> {
        Self::new(models, sockets_connections, Cartesian3D {})
    }
}

impl<T: DirectionSet> Rules<T> {
    fn new(
        models: Vec<NodeModel<T>>,
        sockets_connections: Vec<SocketConnections>,
        direction_set: T,
    ) -> Rules<T> {
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
                                    match unique_models.insert(*allowed_model) {
                                        true => allowed_neighbours
                                            [(model_index, direction as usize)]
                                            .push(*allowed_model),
                                        false => (),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Rules {
            models: expanded_models,
            allowed_neighbours,
            typestate: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn supported_models(
        &self,
        model_index: ModelIndex,
        direction: Direction,
    ) -> &Vec<ModelIndex> {
        &self.allowed_neighbours[(model_index, direction as usize)]
    }

    #[inline]
    pub(crate) fn weight(&self, model_index: ModelIndex) -> f32 {
        self.models[model_index].weight()
    }

    #[inline]
    pub fn models_count(&self) -> usize {
        self.models.len()
    }

    #[inline]
    pub(crate) fn model(&self, index: usize) -> &ExpandedNodeModel {
        &self.models[index]
    }
}

/// Expand sockets connections. `socket_to_sockets`: from a socket, get all sockets that are compatible for connection
fn expand_sockets_connections(
    sockets_connections: Vec<(u32, Vec<u32>)>,
) -> HashMap<u32, HashSet<u32>> {
    let mut socket_to_sockets = HashMap::new();
    for (socket, connections) in sockets_connections {
        {
            let connectable_sockets = socket_to_sockets.entry(socket).or_insert(HashSet::new());
            for other_socket in &connections {
                connectable_sockets.insert(*other_socket);
            }
        }
        // Register the connection from the other socket too.
        for other_socket in &connections {
            if *other_socket != socket {
                let other_connectable_sockets = socket_to_sockets
                    .entry(*other_socket)
                    .or_insert(HashSet::new());
                other_connectable_sockets.insert(socket);
            }
        }
    }
    socket_to_sockets
}
