#[derive(Clone, Copy)]
pub enum Direction {
    XForward = 0,
    XBackward = 1,
    YForward = 2,
    YBackward = 3,
    ZForward = 4,
    ZBackward = 5,
}

pub struct GridDelta {
    dx: i8,
    dy: i8,
    dz: i8,
}

impl GridDelta {
    pub fn new(dx: i8, dy: i8, dz: i8) -> Self {
        Self { dx, dy, dz }
    }
}

pub struct Grid {
    size_x: usize,
    size_y: usize,
    size_z: usize,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    direction_set: &'static [GridDelta],
}

impl Grid {
    pub fn new_cartesian_2d_grid(size_x: usize, size_y: usize, looping: bool) -> Self {
        Self {
            size_x,
            size_y,
            size_z: 1,
            looping_x: looping,
            looping_y: looping,
            looping_z: false,
            direction_set: CARTESIAN_2D_DELTAS,
        }
    }

    pub fn new_cartesian_3d_grid(
        size_x: usize,
        size_y: usize,
        size_z: usize,
        looping: bool,
    ) -> Self {
        Self {
            size_x,
            size_y,
            size_z,
            looping_x: looping,
            looping_y: looping,
            looping_z: looping,
            direction_set: CARTESIAN_3D_DELTAS,
        }
    }

    pub fn total_size(&self) -> usize {
        self.size_x * self.size_y * self.size_z
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
