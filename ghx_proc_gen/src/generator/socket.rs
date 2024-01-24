use std::collections::{HashMap, HashSet};

use crate::grid::direction::{Cartesian2D, Cartesian3D};

use super::model::{ModelRotation, ModelTemplate, ALL_MODEL_ROTATIONS};

/// Id of a possible connection type
pub(crate) type SocketId = u64;

/// Used to create one or more [`Socket`]. Created [`Socket`] can then be used to define [`Model`] and define connections between them.
#[derive(Clone)]
pub struct SocketCollection {
    incremental_socket_index: u32,

    /// For uniqueness
    uniques: HashMap<SocketId, HashSet<SocketId>>,
    /// For determinism and sequential access
    compatibles: HashMap<SocketId, Vec<SocketId>>,
}

impl SocketCollection {
    /// Creates a new [`SocketCollection`]
    pub fn new() -> Self {
        Self {
            incremental_socket_index: 0,
            uniques: HashMap::new(),
            compatibles: HashMap::new(),
        }
    }

    /// Creates a new [`Socket`] in the collection and returns it
    pub fn create(&mut self) -> Socket {
        let socket = Socket::new(self.incremental_socket_index);
        self.incremental_socket_index += 1;
        socket
    }

    /// Adds a connection between two sockets. [`Model`] with sockets `from` can connect to [`Model`] with sockets `to` and vice versa.
    ///
    /// - There is **no** direction in the relation, adding a connection from`a` to `b` also adds a connection from `b` to `a`
    /// - By default (until the connection is explicitly added), a socket is not "compatible" with itself.
    /// ### Example
    /// ```
    /// use ghx_proc_gen::generator::socket::SocketCollection;
    ///
    /// let mut sockets = SocketCollection::new();
    /// let a = sockets.create();
    /// let b = sockets.create();
    /// sockets.add_connection(a, vec![a, b]);
    /// // `a` can be connected to `a` and `b`
    /// // `b` can be connected to `a`
    /// ```
    pub fn add_connection<I>(&mut self, from: Socket, to: I) -> &mut Self
    where
        I: IntoIterator<Item = Socket>,
    {
        for to_socket in to.into_iter() {
            self.register_connection(&from, &to_socket);
        }
        self
    }

    /// Same as `add_connection` but accept multiple connections definitions at the same time.
    /// ### Example
    /// ```
    /// use ghx_proc_gen::generator::socket::SocketCollection;
    ///
    /// let mut sockets = SocketCollection::new();
    /// let (a, b, c) = (sockets.create(), sockets.create(), sockets.create());
    /// sockets.add_connections(vec![
    ///     (a, vec![a, b]),
    ///     (b, vec![c])
    /// ]);
    /// // `a` can be connected to `a` and `b`
    /// // `b` can be connected to `a` and `c`
    /// // `c` can be connected to `b`
    /// ```
    pub fn add_connections<I, J>(&mut self, connections: I) -> &mut Self
    where
        I: IntoIterator<Item = (Socket, J)>,
        J: IntoIterator<Item = Socket>,
    {
        for (from, to_sockets) in connections.into_iter() {
            for to in to_sockets.into_iter() {
                self.register_connection(&from, &to);
            }
        }
        self
    }

    /// Adds a connection between all possible rotations of two sockets that are on the rotation axis of the [`super::Rules`]. [`Model`] with sockets `from` can connect to [`Model`] with sockets `to` and vice versa.
    ///
    /// - There is **no** direction in the relation, adding a connection from`a` to `b` also adds a connection from `b` to `a`
    /// - By default (until the connection is explicitly added), a socket is not "compatible" with itself.
    /// ### Example
    /// ```
    /// use ghx_proc_gen::generator::socket::{SocketCollection, SocketsCartesian3D};
    ///
    /// let mut sockets = SocketCollection::new();
    /// let (side_a, vertical_a) = (sockets.create(), sockets.create());
    /// let (side_b, vertical_b) = (sockets.create(), sockets.create());
    /// // If Y+ is our rotation axis. We could have such node models:
    /// let model_a = SocketsCartesian3D::Simple {
    ///     x_pos: side_a,
    ///     x_neg: side_a,
    ///     z_pos: side_a,
    ///     z_neg: side_a,
    ///     y_pos: vertical_a,
    ///     y_neg: vertical_a,
    /// }.new_model().with_all_rotations();
    /// let model_b = SocketsCartesian3D::Simple {
    ///     x_pos: side_b,
    ///     x_neg: side_b,
    ///     z_pos: side_b,
    ///     z_neg: side_b,
    ///     y_pos: vertical_b,
    ///     y_neg: vertical_b,
    /// }.new_model().with_all_rotations();
    /// sockets.add_rotated_connection(vertical_a, vec![vertical_b]);
    /// // `model_a` and `model_b` can now be stacked on top of each other (no matter their rotations)
    /// // Note: here two `model_a` cannot be stacked on top of each other since `vertical_a` was not said to be connected to itself.
    /// ```
    pub fn add_rotated_connection(&mut self, from: Socket, to: Vec<Socket>) -> &mut Self {
        for to_rotation in ALL_MODEL_ROTATIONS {
            let to_rotated_sockets: Vec<Socket> =
                to.iter().map(|s| s.rotated(*to_rotation)).collect();
            for from_rot in ALL_MODEL_ROTATIONS {
                let rotated_socket = from.rotated(*from_rot);
                for to_socket in to_rotated_sockets.iter() {
                    self.register_connection(&rotated_socket, &to_socket);
                }
            }
        }
        self
    }

    /// Same as `add_rotated_connection` but accepts multiple connections definitions at the same time.
    pub fn add_rotated_connections<I>(&mut self, connections: I) -> &mut Self
    where
        I: IntoIterator<Item = (Socket, Vec<Socket>)>,
    {
        for (from, to_sockets) in connections.into_iter() {
            self.add_rotated_connection(from, to_sockets);
        }
        self
    }

    /// Similar to `add_rotated_connection` but with additional constraints.
    ///
    /// Adds a connection between only the specified `relative_rotations` of two sockets that are on the rotation axis of the [`super::Rules`]. [`Model`] with sockets `from`, with a given relative rotation to socket `to`, can connect to [`Model`] with sockets `to` (and vice versa with the opposite relative rotation).
    ///
    /// `relative_rotations` should be defined with regard to rotation [`ModelRotation::Rot0`] of `to`. So a value of [`ModelRotation::Rot90`] in `relative_rotations` means that a `from` socket can be connected to a `to` socket if and only if the `from` socket is rotated 90° more than the `to` socket, no matter their absolute rotations.
    ///
    /// - There is **no** direction in the relation, adding a connection from`a` to `b` also adds a connection from `b` to `a` (here with the opposite relative rotation)
    /// - By default (until the connection is explicitly added), a socket is not "compatible" with itself.
    pub fn add_constrained_rotated_connection(
        &mut self,
        from: Socket,
        mut relative_rotations: Vec<ModelRotation>,
        to: Vec<Socket>,
    ) -> &mut Self {
        for to_rotation in ALL_MODEL_ROTATIONS {
            let to_rotated_sockets: Vec<Socket> =
                to.iter().map(|s| s.rotated(*to_rotation)).collect();
            for from_rotation in relative_rotations.iter_mut() {
                let from_rotated_socket = from.rotated(*from_rotation);
                for to_socket in to_rotated_sockets.iter() {
                    self.register_connection(&from_rotated_socket, &to_socket);
                }
                *from_rotation = from_rotation.next();
            }
        }
        self
    }

    fn register_connection_half(&mut self, from: &Socket, to: &Socket) {
        // TODO Decide if we check for existence
        let connectable_sockets = self.uniques.entry(from.id()).or_insert(HashSet::new());

        if connectable_sockets.insert(to.id()) {
            self.compatibles
                .entry(from.id())
                .or_insert(Vec::new())
                .push(to.id());
        }
    }

    fn register_connection(&mut self, from: &Socket, to: &Socket) {
        self.register_connection_half(from, to);
        self.register_connection_half(to, from);
    }

    pub(crate) fn get_compatibles(&self, socket: SocketId) -> Option<&Vec<SocketId>> {
        self.compatibles.get(&socket)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.incremental_socket_index == 0
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
/// Defines a contact point of a [`Model`]. Each [`Model`] may have none or multiple socket(s) on each of his sides.
/// Relations between sockets are not defined on the socket nor on the [`Model`] but rather in a [`SocketCollection`].
pub struct Socket {
    /// Index of the socket. Always unique, except for rotated sockets on the rotation axis which share the same `socket_index`
    socket_index: u32,
    /// Internal index which defines the rotation of the socket. Always [`ModelRotation::Rot0`] for sockets that are not on the rotation axis of the [`crate::generator::Rules`]
    rot: ModelRotation,
}

impl Socket {
    pub(crate) fn new(socket_index: u32) -> Self {
        Self {
            socket_index,
            rot: ModelRotation::Rot0,
        }
    }

    pub(crate) fn id(&self) -> SocketId {
        self.socket_index as u64 + ((self.rot.index() as u64) << 32)
    }

    pub(crate) fn rotated(&self, rotation: ModelRotation) -> Socket {
        let mut rotated_socket = self.clone();
        rotated_socket.rot = rotated_socket.rot.rotated(rotation);
        rotated_socket
    }

    pub(crate) fn rotate(&mut self, rotation: ModelRotation) {
        self.rot.rotate(rotation);
    }
}

/// Sockets for a model to be used in a 2d cartesian grid.
pub enum SocketsCartesian2D {
    /// The model has only 1 socket, and its is the same in all directions.
    Mono(Socket),
    /// The model has 1 socket per side.
    Simple {
        /// socket on the x+ side
        x_pos: Socket,
        /// socket on the x- side
        x_neg: Socket,
        /// socket on the y+ side
        y_pos: Socket,
        /// socket on the y- side
        y_neg: Socket,
    },
    /// The model has multiple sockets per side.
    Multiple {
        /// sockets on the x+ side
        x_pos: Vec<Socket>,
        /// sockets on the x- side
        x_neg: Vec<Socket>,
        /// sockets on the y+ side
        y_pos: Vec<Socket>,
        /// sockets on the y- side
        y_neg: Vec<Socket>,
    },
}

impl Into<Vec<Vec<Socket>>> for SocketsCartesian2D {
    fn into(self) -> Vec<Vec<Socket>> {
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
    /// Creates a [`ModelTemplate`] from its sockets definition, with default values for the other members: weight is [`super::model::DEFAULT_MODEL_WEIGHT`] and the model will not be rotated.
    pub fn to_template(self) -> ModelTemplate<Cartesian2D> {
        ModelTemplate::<Cartesian2D>::new(self)
    }
}

impl Into<ModelTemplate<Cartesian2D>> for SocketsCartesian2D {
    fn into(self) -> ModelTemplate<Cartesian2D> {
        self.to_template()
    }
}

/// Sockets for a model to be used in a 3d cartesian grid.
pub enum SocketsCartesian3D {
    /// The model has only 1 socket, and its is the same in all directions.
    Mono(Socket),
    /// The model has 1 socket per side.
    Simple {
        /// socket on the x+ side
        x_pos: Socket,
        /// socket on the x- side
        x_neg: Socket,
        /// socket on the z+ side
        z_pos: Socket,
        /// socket on the z- side
        z_neg: Socket,
        /// socket on the y+ side
        y_pos: Socket,
        /// socket on the y- side
        y_neg: Socket,
    },
    /// The model has multiple sockets per side.
    Multiple {
        /// sockets on the x+ side
        x_pos: Vec<Socket>,
        /// sockets on the x- side
        x_neg: Vec<Socket>,
        /// sockets on the z+ side
        z_pos: Vec<Socket>,
        /// sockets on the z- side
        z_neg: Vec<Socket>,
        /// sockets on the y+ side
        y_pos: Vec<Socket>,
        /// sockets on the y- side
        y_neg: Vec<Socket>,
    },
}

impl Into<Vec<Vec<Socket>>> for SocketsCartesian3D {
    fn into(self) -> Vec<Vec<Socket>> {
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

impl Into<ModelTemplate<Cartesian3D>> for SocketsCartesian3D {
    fn into(self) -> ModelTemplate<Cartesian3D> {
        self.to_template()
    }
}

impl SocketsCartesian3D {
    /// Creates a [`ModelTemplate`] from its sockets definition, with default values for the other members: weight is [`super::model::DEFAULT_MODEL_WEIGHT`] and the model will not be rotated.
    pub fn to_template(self) -> ModelTemplate<Cartesian3D> {
        ModelTemplate::<Cartesian3D>::new(self)
    }
}
