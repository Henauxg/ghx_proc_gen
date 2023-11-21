// TODO See if std::ops::index can be used here
#[derive(Clone, Copy)]
pub enum Direction {
    XForward = 0,
    XBackward = 1,
    YForward = 2,
    YBackward = 3,
    ZForward = 4,
    ZBackward = 5,
}
impl Direction {
    pub(crate) fn opposite(&self) -> Direction {
        match self {
            Direction::XForward => Direction::XBackward,
            Direction::XBackward => Direction::XForward,
            Direction::YForward => Direction::YBackward,
            Direction::YBackward => Direction::YForward,
            Direction::ZForward => Direction::ZBackward,
            Direction::ZBackward => Direction::ZForward,
        }
    }
}

pub struct DirectionSet {
    pub(crate) dirs: &'static [Direction],
    pub(crate) deltas: &'static [GridDelta],
}

pub const CARTESIAN_2D: DirectionSet = DirectionSet {
    dirs: CARTESIAN_2D_DIRECTIONS,
    deltas: CARTESIAN_2D_DELTAS,
};

pub const CARTESIAN_2D_DIRECTIONS: &'static [Direction] = &[
    Direction::XForward,
    Direction::XBackward,
    Direction::YForward,
    Direction::YBackward,
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

pub const CARTESIAN_2D_DELTAS: &'static [GridDelta] = &[
    GridDelta {
        // XForward
        dx: 1,
        dy: 0,
        dz: 0,
    },
    GridDelta {
        // XBackward
        dx: -1,
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
        // YBackward
        dx: 0,
        dy: -1,
        dz: 0,
    },
];

pub const CARTESIAN_3D: DirectionSet = DirectionSet {
    dirs: CARTESIAN_3D_DIRECTIONS,
    deltas: CARTESIAN_3D_DELTAS,
};

pub const CARTESIAN_3D_DIRECTIONS: &'static [Direction] = &[
    Direction::XForward,
    Direction::XBackward,
    Direction::YForward,
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
        // XBackward
        dx: -1,
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
