use self::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet, GridDelta};

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

#[derive(Clone)]
pub struct Grid<T: DirectionSet + Clone> {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    pub(crate) direction_set: T,
    size_xy: u32,
}

impl Grid<Cartesian2D> {
    pub fn new_cartesian_2d(size_x: u32, size_y: u32, looping: bool) -> Grid<Cartesian2D> {
        Self::new(size_x, size_y, 1, looping, looping, false, Cartesian2D {})
    }
}

impl Grid<Cartesian3D> {
    pub fn new_cartesian_3d(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping: bool,
    ) -> Grid<Cartesian3D> {
        Self::new(
            size_x,
            size_y,
            size_z,
            looping,
            looping,
            looping,
            Cartesian3D {},
        )
    }

    pub fn size_z(&self) -> u32 {
        self.size_z
    }
}

impl<T: DirectionSet + Clone> Grid<T> {
    pub fn new(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping_x: bool,
        looping_y: bool,
        looping_z: bool,
        direction_set: T,
    ) -> Grid<T> {
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

    pub fn size_x(&self) -> u32 {
        self.size_x
    }
    pub fn size_y(&self) -> u32 {
        self.size_y
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
        let delta = &self.direction_set.deltas()[direction as usize];
        match self.get_next_pos(grid_position, &delta) {
            Some(next_pos) => Some(self.get_index(&next_pos)),
            None => None,
        }
    }

    #[inline]
    pub(crate) fn directions(&self) -> &'static [Direction] {
        self.direction_set.directions()
    }
}

pub struct GridData<T: DirectionSet + Clone, D> {
    grid: Grid<T>,
    data: Vec<D>,
}

impl<T: DirectionSet + Clone, D> GridData<T, D> {
    pub fn new(grid: Grid<T>, data: Vec<D>) -> Self {
        Self { grid, data }
    }

    pub fn grid(&self) -> &Grid<T> {
        &self.grid
    }

    pub fn set(&mut self, index: usize, val: D) {
        self.data[index] = val;
    }
}

impl<D> GridData<Cartesian2D, D> {
    pub fn get_2d(&self, x: u32, y: u32) -> &D {
        &self.data[(x + y * self.grid.size_x) as usize]
    }
}
