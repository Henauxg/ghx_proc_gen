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

pub struct GridDelta {
    dx: i32,
    dy: i32,
    dz: i32,
}

impl GridDelta {
    pub fn new(dx: i32, dy: i32, dz: i32) -> Self {
        Self { dx, dy, dz }
    }
}

pub struct GridPosition {
    x: u32,
    y: u32,
    z: u32,
}
impl GridPosition {
    fn get_delta_position(&self, delta: &GridDelta) -> (i64, i64, i64) {
        (
            i64::from(self.x) + i64::from(delta.dx),
            i64::from(self.y) + i64::from(delta.dy),
            i64::from(self.z) + i64::from(delta.dz),
        )
    }
}

pub struct Grid {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    // pub(crate) direction_set: &'static [GridDelta],
    pub(crate) direction_set: DirectionSet,

    size_xy: u32,
}

impl Grid {
    pub fn new(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping_x: bool,
        looping_y: bool,
        looping_z: bool,
        direction_set: DirectionSet,
    ) -> Self {
        Self {
            size_x,
            size_y,
            size_z,
            looping_x,
            looping_y,
            looping_z,
            direction_set,
            size_xy: size_x * size_y,
        }
    }

    pub fn new_cartesian_2d(size_x: u32, size_y: u32, looping: bool) -> Self {
        Self::new(size_x, size_y, 1, looping, looping, false, CARTESIAN_2D)
    }

    pub fn new_cartesian_3d(size_x: u32, size_y: u32, size_z: u32, looping: bool) -> Self {
        Self::new(
            size_x,
            size_y,
            size_z,
            looping,
            looping,
            looping,
            CARTESIAN_3D,
        )
    }

    pub fn total_size(&self) -> usize {
        (self.size_x * self.size_y * self.size_z)
            .try_into()
            .unwrap()
    }

    pub(crate) fn get_index(&self, grid_position: &GridPosition) -> usize {
        (grid_position.x + grid_position.y * self.size_x + grid_position.z * self.size_xy)
            .try_into()
            .unwrap()
    }

    pub(crate) fn get_position(&self, grid_index: usize) -> GridPosition {
        let index = u32::try_from(grid_index).unwrap();
        GridPosition {
            x: index % self.size_x,
            y: (index / self.size_x) % self.size_y,
            z: index / self.size_xy,
        }
    }

    pub fn get_next_pos(
        &self,
        grid_position: &GridPosition,
        delta: &GridDelta,
    ) -> Option<GridPosition> {
        let mut next_pos = grid_position.get_delta_position(&delta);
        for (looping, pos, size) in vec![
            (self.looping_x, &mut next_pos.0, self.size_x),
            (self.looping_y, &mut next_pos.1, self.size_y),
            (self.looping_z, &mut next_pos.2, self.size_z),
        ] {
            match looping {
                true => {
                    if *pos < 0 {
                        *pos += size as i64
                    }
                    if *pos >= size as i64 {
                        *pos -= size as i64
                    }
                }
                false => {
                    if *pos < 0 || *pos >= size as i64 {
                        return None;
                    }
                }
            }
        }
        Some(GridPosition {
            x: u32::try_from(next_pos.0).unwrap(),
            y: u32::try_from(next_pos.1).unwrap(),
            z: u32::try_from(next_pos.2).unwrap(),
        })
    }

    pub fn get_next_index(
        &self,
        grid_position: &GridPosition,
        direction: Direction,
    ) -> Option<usize> {
        let delta = &self.direction_set.deltas[direction as usize];
        match self.get_next_pos(grid_position, &delta) {
            Some(next_pos) => Some(self.get_index(&next_pos)),
            None => None,
        }
    }

    #[inline]
    pub(crate) fn directions(&self) -> &'static [Direction] {
        self.direction_set.dirs
    }
}

pub struct DirectionSet {
    dirs: &'static [Direction],
    deltas: &'static [GridDelta],
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
