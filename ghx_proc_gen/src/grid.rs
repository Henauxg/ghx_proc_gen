use std::ops::Range;

use self::direction::{Cartesian2D, Cartesian3D, Direction, DirectionSet, GridDelta};

pub mod direction;

#[derive(Debug)]
pub struct GridPosition {
    pub x: u32,
    pub y: u32,
    pub z: u32,
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
pub struct GridDefinition<T: DirectionSet + Clone> {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    pub(crate) direction_set: T,
    size_xy: u32,
}

impl GridDefinition<Cartesian2D> {
    pub fn new_cartesian_2d(
        size_x: u32,
        size_y: u32,
        looping: bool,
    ) -> GridDefinition<Cartesian2D> {
        Self::new(size_x, size_y, 1, looping, looping, false, Cartesian2D {})
    }

    /// Returns the index from a grid position, ignoring the Z axis.
    ///
    ///  NO CHECK is done to verify that the given position is a valid position for this grid.
    #[inline]
    pub fn get_index_2d(&self, x: u32, y: u32) -> usize {
        (x + y * self.size_x).try_into().unwrap()
    }

    /// Returns the index from a grid position, ignoring the Z axis.
    ///
    ///  NO CHECK is done to verify that the given position is a valid position for this grid.
    #[inline]
    pub fn get_index_from_pos_2d(&self, grid_position: &GridPosition) -> usize {
        self.get_index_2d(grid_position.x, grid_position.y)
    }
}

impl GridDefinition<Cartesian3D> {
    pub fn new_cartesian_3d(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping: bool,
    ) -> GridDefinition<Cartesian3D> {
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

    /// Returns the size of the grid in the Z axis.
    pub fn size_z(&self) -> u32 {
        self.size_z
    }
}

impl<T: DirectionSet + Clone> GridDefinition<T> {
    /// Creates a new [`GridDefinition`]
    pub fn new(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping_x: bool,
        looping_y: bool,
        looping_z: bool,
        direction_set: T,
    ) -> GridDefinition<T> {
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

    /// Returns the size of the grid in the X axis.
    pub fn size_x(&self) -> u32 {
        self.size_x
    }

    /// Returns the size of the grid in the Y axis.
    pub fn size_y(&self) -> u32 {
        self.size_y
    }

    /// Returns the total size of the grid
    pub fn total_size(&self) -> usize {
        (self.size_x * self.size_y * self.size_z)
            .try_into()
            .unwrap()
    }

    pub fn indexes(&self) -> Range<usize> {
        0..self.total_size()
    }

    /// Returns the index from a grid position.
    ///
    ///  NO CHECK is done to verify that the given position is a valid position for this grid.
    #[inline]
    pub fn get_index(&self, x: u32, y: u32, z: u32) -> usize {
        (x + y * self.size_x + z * self.size_xy).try_into().unwrap()
    }

    /// Returns the index from a grid position.
    ///
    ///  NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_index_from_pos(&self, grid_position: &GridPosition) -> usize {
        self.get_index(grid_position.x, grid_position.y, grid_position.z)
    }

    pub fn get_position(&self, grid_index: usize) -> GridPosition {
        let index = u32::try_from(grid_index).unwrap();
        GridPosition {
            x: index % self.size_x,
            y: (index / self.size_x) % self.size_y,
            z: index / self.size_xy,
        }
    }

    /// Returns the next position in the grid when moving `delta` unit(s) in `direction` from `grid_position`.
    ///
    /// Returns `None` if the destination is not in the grid.
    ///
    /// NO CHECK is done to verify that the given `grid_position` is a valid position for this grid.
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

    /// Returns the index of the next position in the grid when moving 1 unit in `direction` from `grid_position`.
    ///
    /// Returns `None` if the destination is not in the grid.
    ///
    /// NO CHECK is done to verify that the given `grid_position` is a valid position for this grid.
    pub fn get_next_index(
        &self,
        grid_position: &GridPosition,
        direction: Direction,
    ) -> Option<usize> {
        let delta = &self.direction_set.deltas()[direction as usize];
        match self.get_next_pos(grid_position, &delta) {
            Some(next_pos) => Some(self.get_index_from_pos(&next_pos)),
            None => None,
        }
    }

    #[inline]
    pub(crate) fn directions(&self) -> &'static [Direction] {
        self.direction_set.directions()
    }

    /// Creates a default [`GridData`] with the size of the [`GridDefinition`] with each element value set to its default one.
    pub fn default_grid_data<D: Default + Clone>(&self) -> GridData<T, D> {
        GridData {
            grid: self.clone(),
            data: vec![D::default(); self.total_size()],
        }
    }

    /// Creates a [`GridData`] with the size of the [`GridDefinition`] with each element value being a copy of the given one.
    pub fn new_grid_data<D: Clone>(&self, element: D) -> GridData<T, D> {
        GridData {
            grid: self.clone(),
            data: vec![element; self.total_size()],
        }
    }
}

/// Holds a [`GridDefinition`] and generic data in a linear buffer that can be accessed through the grid definition to represent the grid content.
/// ### Example
///
/// Create a default `GridData` from a `GridDefinition`
/// ```
/// use ghx_proc_gen::grid::GridDefinition;
///
/// let grid = GridDefinition::new_cartesian_2d(10, 10, false);
/// let grid_data = grid.default_grid_data::<u64>();
/// ```
/// You can also retrieve a pre-created existing `GridData` from a [`ghx_proc_gen::Generator`], or from an observer like a [`ghx_proc_gen::generator::QueuedStatefulObserver`]
pub struct GridData<T: DirectionSet + Clone, D> {
    grid: GridDefinition<T>,
    data: Vec<D>,
}

impl<T: DirectionSet + Clone, D> GridData<T, D> {
    /// Prefer using `default_grid_data` or `new_grid_data` directly on an existing grid definition to create a `GridData` with a correct data Vec.
    pub fn new(grid: GridDefinition<T>, data: Vec<D>) -> Self {
        Self { grid, data }
    }

    /// Returns a reference to the `GridDefinition` this is based on
    pub fn grid(&self) -> &GridDefinition<T> {
        &self.grid
    }

    /// Sets the value of the element at `index` in the grid.
    ///
    /// NO CHECK is done to verify that the given index is a valid index for this grid.
    pub fn set(&mut self, index: usize, val: D) {
        self.data[index] = val;
    }

    /// Returns a reference to the element at this index.
    ///
    /// NO CHECK is done to verify that the given index is a valid index for this grid.
    pub fn get(&self, index: usize) -> &D {
        &self.data[index]
    }

    /// Returns a mutable reference to the element at this index.
    ///
    /// NO CHECK is done to verify that the given index is a valid index for this grid.
    pub fn get_mut(&mut self, index: usize) -> &mut D {
        &mut self.data[index]
    }

    pub fn nodes(&self) -> &Vec<D> {
        &self.data
    }
}

impl<D> GridData<Cartesian2D, D> {
    /// Returns a reference to the element at this position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_2d(&self, x: u32, y: u32) -> &D {
        &self.data[self.grid.get_index_2d(x, y)]
    }

    /// Returns a mutable reference to the data at this position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_2d_mut(&mut self, x: u32, y: u32) -> &mut D {
        &mut self.data[self.grid.get_index_2d(x, y)]
    }
}

impl<D> GridData<Cartesian3D, D> {
    /// Returns a reference to the data at this position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_3d(&self, x: u32, y: u32, z: u32) -> &D {
        &self.data[self.grid.get_index(x, y, z)]
    }

    /// Returns a mutable reference to the data at this position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_3d_mut(&mut self, x: u32, y: u32, z: u32) -> &mut D {
        &mut self.data[self.grid.get_index(x, y, z)]
    }
}
