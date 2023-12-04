// TODO See if std::ops::index can be used here
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    XForward = 0,
    YForward = 1,
    XBackward = 2,
    YBackward = 3,
    ZForward = 4,
    ZBackward = 5,
}
impl Direction {
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

pub struct GridDelta {
    pub(crate) dx: i32,
    pub(crate) dy: i32,
    pub(crate) dz: i32,
}

impl GridDelta {
    pub fn new(dx: i32, dy: i32, dz: i32) -> Self {
        Self { dx, dy, dz }
    }
}

pub trait DirectionSet {
    fn directions(&self) -> &'static [Direction];
    fn deltas(&self) -> &'static [GridDelta];
}

#[derive(Clone)]
pub struct Cartesian2D {}
impl DirectionSet for Cartesian2D {
    fn directions(&self) -> &'static [Direction] {
        CARTESIAN_2D_DIRECTIONS
    }

    fn deltas(&self) -> &'static [GridDelta] {
        CARTESIAN_2D_DELTAS
    }
}

#[derive(Clone)]
pub struct Cartesian3D {}
impl DirectionSet for Cartesian3D {
    fn directions(&self) -> &'static [Direction] {
        CARTESIAN_3D_DIRECTIONS
    }

    fn deltas(&self) -> &'static [GridDelta] {
        CARTESIAN_3D_DELTAS
    }
}

pub const CARTESIAN_2D_DIRECTIONS: &'static [Direction] = &[
    Direction::XForward,
    Direction::YForward,
    Direction::XBackward,
    Direction::YBackward,
];
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

pub const CARTESIAN_3D_DIRECTIONS: &'static [Direction] = &[
    Direction::XForward,
    Direction::YForward,
    Direction::XBackward,
    Direction::YBackward,
    Direction::ZForward,
    Direction::ZBackward,
];
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
        // ZBackward
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
