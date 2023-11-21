use self::direction::{Direction, DirectionSet, GridDelta, CARTESIAN_2D, CARTESIAN_3D};

pub mod direction;

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

pub trait GridTrait {
    fn total_size(&self) -> usize;
    fn get_index(&self, grid_position: &GridPosition) -> usize;
    fn get_position(&self, grid_index: usize) -> GridPosition;
    fn get_next_pos(&self, grid_position: &GridPosition, delta: &GridDelta)
        -> Option<GridPosition>;
    fn get_next_index(&self, grid_position: &GridPosition, direction: Direction) -> Option<usize>;
    fn directions(&self) -> &'static [Direction];
}

pub struct Grid {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
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

pub struct GridCartesian2D {
    pub(crate) grid: Grid,
}

impl GridCartesian2D {
    pub fn new(size_x: u32, size_y: u32) -> Self {
        Self {
            grid: Grid::new(size_x, size_y, 1, false, false, false, CARTESIAN_2D),
        }
    }

    pub fn new_looping(size_x: u32, size_y: u32, looping: (bool, bool)) -> Self {
        Self {
            grid: Grid::new(size_x, size_y, 1, looping.0, looping.1, false, CARTESIAN_2D),
        }
    }
}

impl GridTrait for GridCartesian2D {
    #[inline]
    fn total_size(&self) -> usize {
        self.grid.total_size()
    }

    #[inline]
    fn get_index(&self, grid_position: &GridPosition) -> usize {
        self.grid.get_index(grid_position)
    }

    #[inline]
    fn get_position(&self, grid_index: usize) -> GridPosition {
        self.grid.get_position(grid_index)
    }

    #[inline]
    fn get_next_pos(
        &self,
        grid_position: &GridPosition,
        delta: &GridDelta,
    ) -> Option<GridPosition> {
        self.grid.get_next_pos(grid_position, delta)
    }

    #[inline]
    fn get_next_index(&self, grid_position: &GridPosition, direction: Direction) -> Option<usize> {
        self.grid.get_next_index(grid_position, direction)
    }

    #[inline]
    fn directions(&self) -> &'static [Direction] {
        self.grid.directions()
    }
}

pub struct GridCartesian3D {
    pub(crate) grid: Grid,
}

impl GridCartesian3D {
    pub fn new(size: (u32, u32, u32)) -> Self {
        Self {
            grid: Grid::new(size.0, size.1, size.2, false, false, false, CARTESIAN_3D),
        }
    }

    pub fn new_looping(size: (u32, u32, u32), looping: (bool, bool, bool)) -> Self {
        Self {
            grid: Grid::new(
                size.0,
                size.1,
                size.2,
                looping.0,
                looping.1,
                looping.2,
                CARTESIAN_3D,
            ),
        }
    }
}

impl GridTrait for GridCartesian3D {
    #[inline]
    fn total_size(&self) -> usize {
        self.grid.total_size()
    }

    #[inline]
    fn get_index(&self, grid_position: &GridPosition) -> usize {
        self.grid.get_index(grid_position)
    }

    #[inline]
    fn get_position(&self, grid_index: usize) -> GridPosition {
        self.grid.get_position(grid_index)
    }

    #[inline]
    fn get_next_pos(
        &self,
        grid_position: &GridPosition,
        delta: &GridDelta,
    ) -> Option<GridPosition> {
        self.grid.get_next_pos(grid_position, delta)
    }

    #[inline]
    fn get_next_index(&self, grid_position: &GridPosition, direction: Direction) -> Option<usize> {
        self.grid.get_next_index(grid_position, direction)
    }

    #[inline]
    fn directions(&self) -> &'static [Direction] {
        self.grid.directions()
    }
}
