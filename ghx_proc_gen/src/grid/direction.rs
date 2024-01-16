// TODO See if std::ops::index can be used here

/// Represents an oriented axis of a coordinate system
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    /// X+ axis
    XForward = 0,
    /// Y+ axis
    YForward = 1,
    /// X- axis
    XBackward = 2,
    /// Y- axis
    YBackward = 3,
    /// Z+ axis
    ZForward = 4,
    /// Z- axis
    ZBackward = 5,
}
impl Direction {
    /// Returns the opposite [`Direction`]
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::XForward => Direction::XBackward,
            Direction::XBackward => Direction::XForward,
            Direction::YForward => Direction::YBackward,
            Direction::YBackward => Direction::YForward,
            Direction::ZForward => Direction::ZBackward,
            Direction::ZBackward => Direction::ZForward,
        }
    }

    pub(crate) fn rotation_basis(&self) -> &'static [Direction] {
        match self {
            Direction::XForward => X_POS_AXIS,
            Direction::XBackward => X_NEG_AXIS,
            Direction::YForward => Y_POS_AXIS,
            Direction::YBackward => Y_NEG_AXIS,
            Direction::ZForward => Z_POS_AXIS,
            Direction::ZBackward => Z_NEG_AXIS,
        }
    }
}

pub(crate) const X_POS_AXIS: &'static [Direction] = &[
    Direction::YForward,
    Direction::ZForward,
    Direction::YBackward,
    Direction::ZBackward,
];
pub(crate) const X_NEG_AXIS: &'static [Direction] = &[
    Direction::ZForward,
    Direction::YForward,
    Direction::ZBackward,
    Direction::YBackward,
];
pub(crate) const Y_POS_AXIS: &'static [Direction] = &[
    Direction::ZForward,
    Direction::XForward,
    Direction::ZBackward,
    Direction::XBackward,
];
pub(crate) const Y_NEG_AXIS: &'static [Direction] = &[
    Direction::XForward,
    Direction::ZForward,
    Direction::XBackward,
    Direction::ZBackward,
];
pub(crate) const Z_POS_AXIS: &'static [Direction] = &[
    Direction::XForward,
    Direction::YForward,
    Direction::XBackward,
    Direction::YBackward,
];
pub(crate) const Z_NEG_AXIS: &'static [Direction] = &[
    Direction::YForward,
    Direction::XForward,
    Direction::YBackward,
    Direction::XBackward,
];

/// Represents a displacement on a grid
#[derive(Clone, Default, Eq, PartialEq)]
pub struct GridDelta {
    /// Amount of movement on the X axis
    pub dx: i32,
    /// Amount of movement on the Y axis
    pub dy: i32,
    /// Amount of movement on the Z axis
    pub dz: i32,
}

impl GridDelta {
    /// Creates a new [`GridDelta`]
    pub fn new(dx: i32, dy: i32, dz: i32) -> Self {
        Self { dx, dy, dz }
    }
}

/// Represents a coordinate system
pub trait CoordinateSystem: Clone + Sync + Send + 'static {
    /// Returns the [`Direction`] in this coordinate system
    fn directions(&self) -> &'static [Direction];
    /// Returns the [`GridDelta`] for each direction in this coordinate system
    fn deltas(&self) -> &'static [GridDelta];
}

/// Right-handed 2d Cartesian coordinate system: 4 directions
#[derive(Clone)]
pub struct Cartesian2D;
impl CoordinateSystem for Cartesian2D {
    fn directions(&self) -> &'static [Direction] {
        CARTESIAN_2D_DIRECTIONS
    }

    fn deltas(&self) -> &'static [GridDelta] {
        CARTESIAN_2D_DELTAS
    }
}

/// Right-handed 3d Cartesian coordinate system: 6 directions
#[derive(Clone)]
pub struct Cartesian3D;
impl CoordinateSystem for Cartesian3D {
    fn directions(&self) -> &'static [Direction] {
        CARTESIAN_3D_DIRECTIONS
    }

    fn deltas(&self) -> &'static [GridDelta] {
        CARTESIAN_3D_DELTAS
    }
}

/// All the directions that forms a 2d cartesian coordinate system
pub const CARTESIAN_2D_DIRECTIONS: &'static [Direction] = &[
    Direction::XForward,
    Direction::YForward,
    Direction::XBackward,
    Direction::YBackward,
];

/// All the [`GridDelta`], one for each direction, in a cartesian 2d coordinate system
pub const CARTESIAN_2D_DELTAS: &'static [GridDelta] = &[
    GridDelta {
        // XForward
        dx: 1,
        dy: 0,
        dz: 0,
    },
    GridDelta {
        // YForward
        dx: 0,
        dy: 1,
        dz: 0,
    },
    GridDelta {
        // XBackward
        dx: -1,
        dy: 0,
        dz: 0,
    },
    GridDelta {
        // YBackward
        dx: 0,
        dy: -1,
        dz: 0,
    },
];

/// All the directions that forms a 3d cartesian coordinate system
pub const CARTESIAN_3D_DIRECTIONS: &'static [Direction] = &[
    Direction::XForward,
    Direction::YForward,
    Direction::XBackward,
    Direction::YBackward,
    Direction::ZForward,
    Direction::ZBackward,
];

/// All the [`GridDelta`], one for each direction, in a cartesian 3d coordinate system
pub const CARTESIAN_3D_DELTAS: &'static [GridDelta] = &[
    GridDelta {
        // XForward
        dx: 1,
        dy: 0,
        dz: 0,
    },
    GridDelta {
        // YForward
        dx: 0,
        dy: 1,
        dz: 0,
    },
    GridDelta {
        // XBackward
        dx: -1,
        dy: 0,
        dz: 0,
    },
    GridDelta {
        // YBackward
        dx: 0,
        dy: -1,
        dz: 0,
    },
    GridDelta {
        // ZForward
        dx: 0,
        dy: 0,
        dz: 1,
    },
    GridDelta {
        // ZBackward
        dx: 0,
        dy: 0,
        dz: -1,
    },
];
