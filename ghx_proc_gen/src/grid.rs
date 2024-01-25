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

    pub fn new(x: u32, y: u32, z: u32) -> GridPosition {
        Self { x, y, z }
    }

    pub fn new_xy(x: u32, y: u32) -> GridPosition {
        Self { x, y, z: 0 }
    }
}

///
#[derive(Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct GridDefinition<T: CoordinateSystem> {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    pub(crate) coord_system: T,
    /// Cache value of `size_x` * `size_y` for index computations
    size_xy: u32,
}

impl<T: CoordinateSystem> fmt::Display for GridDefinition<T> {
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

impl<T: CoordinateSystem> GridDefinition<T> {
    /// Creates a new [`GridDefinition`]
    pub fn new(
        size_x: u32,
        size_y: u32,
        size_z: u32,
        looping_x: bool,
        looping_y: bool,
        looping_z: bool,
        coord_system: T,
    ) -> GridDefinition<T> {
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
    pub fn get_index(&self, x: u32, y: u32, z: u32) -> NodeIndex {
        (x + y * self.size_x + z * self.size_xy).try_into().unwrap()
    }

    /// Returns the index from a grid position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_index_from_pos(&self, grid_position: &GridPosition) -> NodeIndex {
        self.get_index(grid_position.x, grid_position.y, grid_position.z)
    }

    /// Returns a [`GridPosition`] from the index of a node in this [`GridDefinition`].
    ///
    /// Panics if the index is not a valid index.
    pub fn get_position(&self, grid_index: NodeIndex) -> GridPosition {
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
            Some(next_pos) => Some(self.get_index_from_pos(&next_pos)),
            None => None,
        }
    }

    #[inline]
    pub(crate) fn directions(&self) -> &'static [Direction] {
        self.coord_system.directions()
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
/// let grid = GridDefinition::new_cartesian_2d(10, 10, false, false);
/// let grid_data = grid.default_grid_data::<u64>();
/// ```
/// You can also retrieve a pre-created existing `GridData` from a [`crate::generator::Generator`], or from an observer like a [`crate::generator::observer::QueuedStatefulObserver`]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct GridData<T: CoordinateSystem, D> {
    grid: GridDefinition<T>,
    data: Vec<D>,
}

impl<T: CoordinateSystem, D> GridData<T, D> {
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

    /// Returns a reference to the undelying data buffer.
    pub fn nodes(&self) -> &Vec<D> {
        &self.data
    }
}

impl<T: CoordinateSystem, D: Copy> GridData<T, D> {
    /// Resets the whole grid buffer by setting the value of each element to `value`
    pub fn reset(&mut self, value: D) {
        for d in self.data.iter_mut() {
            *d = value;
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
        &self.data[self.grid.get_index(x, y, z)]
    }

    /// Returns a mutable reference to the data at this position.
    ///
    /// NO CHECK is done to verify that the given position is a valid position for this grid.
    pub fn get_3d_mut(&mut self, x: u32, y: u32, z: u32) -> &mut D {
        &mut self.data[self.grid.get_index(x, y, z)]
    }
}

pub enum NodeRef {
    Index(NodeIndex),
    Pos(GridPosition),
}

impl NodeRef {
    pub fn to_index<T: CoordinateSystem>(&self, grid: &GridDefinition<T>) -> NodeIndex {
        match self {
            NodeRef::Index(index) => *index,
            NodeRef::Pos(pos) => grid.get_index_from_pos(pos),
        }
    }
}

impl Into<NodeRef> for NodeIndex {
    fn into(self) -> NodeRef {
        NodeRef::Index(self)
    }
}
impl Into<NodeRef> for GridPosition {
    fn into(self) -> NodeRef {
        NodeRef::Pos(self)
    }
}
