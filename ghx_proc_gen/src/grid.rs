use std::{fmt, ops::Range};

use self::direction::{Cartesian2D, Cartesian3D, CoordinateSystem, Direction, GridDelta};

#[cfg(feature = "bevy")]
use bevy::ecs::component::Component;

/// Defines directions and coordinate systems
pub mod direction;

/// Index of a Node
pub type NodeIndex = usize;

/// Represents a position in a grid in a practical format
#[derive(Debug, Clone)]
pub struct GridPosition {
    /// Position on the x axis
    pub x: u32,
    /// Position on the y axis
    pub y: u32,
    /// Position on the z axis
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

    /// Utility constructor
    pub fn new(x: u32, y: u32, z: u32) -> GridPosition {
        Self { x, y, z }
    }

    /// Utility constructor for a 2D (x,y) position. z will be set to 0
    pub fn new_xy(x: u32, y: u32) -> GridPosition {
        Self { x, y, z: 0 }
    }
}

/// Definition of a grid
#[derive(Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct GridDefinition<C: CoordinateSystem> {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    pub(crate) coord_system: C,
    /// Cache value of `size_x` * `size_y` for index computations
    size_xy: u32,
}

impl<C: CoordinateSystem> fmt::Display for GridDefinition<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "( size: {} {} {}, looping: {} {} {} )",
            self.size_x, self.size_y, self.size_z, self.looping_x, self.looping_y, self.looping_z
        )
    }
}

impl GridDefinition<Cartesian2D> {
    /// Creates a new grid with a [`Cartesian2D`] coordinate system
    ///
    /// Use `looping` to specify if the coordinates on an axis should loop when reaching the end of the axis.
    pub fn new_cartesian_2d(
        size_x: u32,
        size_y: u32,
        looping_x: bool,
        looping_y: bool,
    ) -> GridDefinition<Cartesian2D> {
        Self::new(size_x, size_y, 1, looping_x, looping_y, false, Cartesian2D)
    }

    /// Returns the index from a grid position, ignoring the Z axis.
    ///
    ///  NO CHECK is done to verify that the given position is a valid position for this grid.
    #[inline]
    pub fn get_index_2d(&self, x: u32, y: u32) -> NodeIndex {
        (x + y * self.size_x).try_into().unwrap()
    }

    /// Returns the index from a grid position, ignoring the Z axis.
    ///
    ///  NO CHECK is done to verify that the given position is a valid position for this grid.
    #[inline]
    pub fn get_index_from_pos_2d(&self, grid_position: &GridPosition) -> NodeIndex {
        self.get_index_2d(grid_position.x, grid_position.y)
    }
}

impl GridDefinition<Cartesian3D> {
    /// Creates a new grid with a [`Cartesian3D`] coordinate system
    ///
    /// Use `looping` to specify if the coordinates on an axis should loop when reaching the end of the axis.
    pub fn new_cartesian_3d(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping_x: bool,
        looping_y: bool,
        looping_z: bool,
    ) -> GridDefinition<Cartesian3D> {
        Self::new(
            size_x,
            size_y,
            size_z,
            looping_x,
            looping_y,
            looping_z,
            Cartesian3D,
        )
    }
}

impl<C: CoordinateSystem> GridDefinition<C> {
    /// Creates a new [`GridDefinition`]
    pub fn new(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping_x: bool,
        looping_y: bool,
        looping_z: bool,
        coord_system: C,
    ) -> GridDefinition<C> {
        Self {
            size_x,
            size_y,
            size_z,
            looping_x,
            looping_y,
            looping_z,
            coord_system,
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

    /// Returns the size of the grid in the Z axis.
    pub fn size_z(&self) -> u32 {
        self.size_z
    }

    /// Returns the size of this grid as a tuple
    pub fn size(&self) -> (u32, u32, u32) {
        (self.size_x, self.size_y, self.size_z)
    }

    /// Returns the total size of the grid
    pub fn total_size(&self) -> usize {
        (self.size_xy * self.size_z).try_into().unwrap()
    }

    /// Returns a [`Range`] over all node indexes in this grid
    pub fn indexes(&self) -> Range<NodeIndex> {
        0..self.total_size()
    }

    /// Returns the index from a grid position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    #[inline]
    pub fn index_from_coords(&self, x: u32, y: u32, z: u32) -> NodeIndex {
        (x + y * self.size_x + z * self.size_xy).try_into().unwrap()
    }

    /// Returns the index from a grid position.
    ///
    /// NO CHECK is done to verify that the given `grid_position` is a valid position for this grid.
    pub fn index_from_pos(&self, grid_position: &GridPosition) -> NodeIndex {
        self.index_from_coords(grid_position.x, grid_position.y, grid_position.z)
    }

    /// Returns a [`GridPosition`] from the index of a node in this [`GridDefinition`].
    ///
    /// Panics if the index is not a valid index.
    pub fn pos_from_index(&self, grid_index: NodeIndex) -> GridPosition {
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
    ) -> Option<NodeIndex> {
        let delta = &self.coord_system.deltas()[direction as usize];
        match self.get_next_pos(grid_position, &delta) {
            Some(next_pos) => Some(self.index_from_pos(&next_pos)),
            None => None,
        }
    }

    /// Returns all the [`Direction`] in the [`CoordinateSystem`] used by this [`GridDefinition`]
    #[inline]
    pub fn directions(&self) -> &'static [Direction] {
        self.coord_system.directions()
    }

    /// Creates a default [`GridData`] with the size of the [`GridDefinition`] with each element value set to its default one.
    pub fn default_grid_data<D: Default + Clone>(&self) -> GridData<C, D> {
        GridData {
            grid: self.clone(),
            data: vec![D::default(); self.total_size()],
        }
    }

    /// Creates a [`GridData`] with the size of the [`GridDefinition`] with each element value being a copy of the given one.
    pub fn new_grid_data<D: Clone>(&self, element: D) -> GridData<C, D> {
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
/// let grid = GridDefinition::new_cartesian_2d(10, 10, false, false);
/// let grid_data = grid.default_grid_data::<u64>();
/// ```
/// You can also retrieve a pre-created existing `GridData` from a [`crate::generator::Generator`], or from an observer like a [`crate::generator::observer::QueuedStatefulObserver`]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct GridData<C: CoordinateSystem, D> {
    grid: GridDefinition<C>,
    data: Vec<D>,
}

impl<C: CoordinateSystem, D> GridData<C, D> {
    /// Prefer using `default_grid_data` or `new_grid_data` directly on an existing grid definition to create a `GridData` with a correct data Vec.
    pub fn new(grid: GridDefinition<C>, data: Vec<D>) -> Self {
        Self { grid, data }
    }

    /// Returns a reference to the `GridDefinition` this is based on
    pub fn grid(&self) -> &GridDefinition<C> {
        &self.grid
    }

    /// Sets the value of the element at `index` in the grid.
    ///
    /// NO CHECK is done to verify that the given index is a valid index for this grid.
    pub fn set(&mut self, index: NodeIndex, value: D) {
        self.data[index] = value;
    }

    /// Returns a reference to the element at this index.
    ///
    /// NO CHECK is done to verify that the given index is a valid index for this grid.
    pub fn get(&self, index: NodeIndex) -> &D {
        &self.data[index]
    }

    /// Returns a mutable reference to the element at this index.
    ///
    /// NO CHECK is done to verify that the given index is a valid index for this grid.
    pub fn get_mut(&mut self, index: NodeIndex) -> &mut D {
        &mut self.data[index]
    }

    /// Returns a reference to the underlying data buffer.
    pub fn nodes(&self) -> &Vec<D> {
        &self.data
    }
}

/// Uses Copy if possible.
impl<C: CoordinateSystem, D: Clone> GridData<C, D> {
    /// Resets the whole grid buffer by setting the value of each element to `value`
    pub fn reset(&mut self, value: D) {
        for d in self.data.iter_mut() {
            *d = value.clone();
        }
    }

    /// Sets all nodes of the grix with x=`x` to `value`
    pub fn set_all_x(&mut self, x: u32, value: D) {
        let mut index = x;
        for _z in 0..self.grid.size_z {
            for _y in 0..self.grid.size_y {
                self.data[index as usize] = value.clone();
                index += self.grid.size_x;
            }
            index += self.grid.size_xy;
        }
    }

    /// Sets all nodes of the grix with y=`y` to `value`
    pub fn set_all_y(&mut self, y: u32, value: D) {
        let mut index = y * self.grid.size_x;
        for _z in 0..self.grid.size_z {
            for _x in 0..self.grid.size_x {
                self.data[index as usize] = value.clone();
                index += 1;
            }
            index += self.grid.size_xy;
        }
    }
    /// Sets all nodes of the grix with z=`z` to `value`
    pub fn set_all_z(&mut self, z: u32, value: D) {
        let mut index = z * self.grid.size_xy;
        for _y in 0..self.grid.size_y {
            for _x in 0..self.grid.size_x {
                self.data[index as usize] = value.clone();
                index += 1;
            }
            index += self.grid.size_x;
        }
    }

    /// Sets all nodes of the grix with x=`x`and y=`y` to `value`
    pub fn set_all_xy(&mut self, x: u32, y: u32, value: D) {
        let mut index = x + y * self.grid.size_x;
        for _z in 0..self.grid.size_z {
            self.data[index as usize] = value.clone();
            index += self.grid.size_xy;
        }
    }

    /// Sets all nodes of the grix with x=`x`and z=`z` to `value`
    pub fn set_all_xz(&mut self, x: u32, z: u32, value: D) {
        let mut index = x + z * self.grid.size_xy;
        for _y in 0..self.grid.size_y {
            self.data[index as usize] = value.clone();
            index += self.grid.size_x;
        }
    }

    /// Sets all nodes of the grix with y=`y` and z=`z` to `value`
    pub fn set_all_yz(&mut self, y: u32, z: u32, value: D) {
        let mut index = y * self.grid.size_x + z * self.grid.size_xy;
        for _x in 0..self.grid.size_x {
            self.data[index as usize] = value.clone();
            index += 1;
        }
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
        &self.data[self.grid.index_from_coords(x, y, z)]
    }

    /// Returns a mutable reference to the data at this position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_3d_mut(&mut self, x: u32, y: u32, z: u32) -> &mut D {
        &mut self.data[self.grid.index_from_coords(x, y, z)]
    }
}

/// Represents a reference to a node of a [`GridDefinition`] or [`GridData`]
pub trait NodeRef<C: CoordinateSystem> {
    /// Returns the [`NodeIndex`] that is referenced by this `NodeRef`.
    fn to_index(&self, grid: &GridDefinition<C>) -> NodeIndex;
}
impl<C: CoordinateSystem> NodeRef<C> for &GridPosition {
    fn to_index(&self, grid: &GridDefinition<C>) -> NodeIndex {
        grid.index_from_pos(self)
    }
}
impl<C: CoordinateSystem> NodeRef<C> for GridPosition {
    fn to_index(&self, grid: &GridDefinition<C>) -> NodeIndex {
        grid.index_from_pos(self)
    }
}
impl<C: CoordinateSystem> NodeRef<C> for NodeIndex {
    fn to_index(&self, _grid: &GridDefinition<C>) -> NodeIndex {
        *self
    }
}
