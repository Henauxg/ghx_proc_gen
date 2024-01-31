use bevy::{
    ecs::{component::Component, system::Query},
    gizmos::gizmos::Gizmos,
    math::{Vec2, Vec3},
    render::color::Color,
    transform::components::Transform,
};
use ghx_proc_gen::grid::GridDefinition;

use super::CoordinateSystem;

/// 3d-specific ([`bevy::prelude::Camera3d`]) configuration of a grid debug view
#[derive(Component)]
pub struct DebugGridViewConfig3d {
    /// Size of a grid node in world units on all 3 axis. Defaults to [`Vec3::ONE`]
    pub node_size: Vec3,
}
impl Default for DebugGridViewConfig3d {
    fn default() -> Self {
        Self {
            node_size: Vec3::ONE,
        }
    }
}

/// 2d-specific ([`bevy::prelude::Camera2d`]) configuration of a grid debug view
#[derive(Component)]
pub struct DebugGridViewConfig2d {
    /// Size of a grid node in world units on the x and y axis. Defaults to 32.0 on both axis
    pub node_size: Vec2,
}
impl Default for DebugGridViewConfig2d {
    fn default() -> Self {
        Self {
            node_size: Vec2::splat(32.),
        }
    }
}

/// Component used on all debug grid to store configuration.
///
/// Updating the component members will update the grid debug view directly
#[derive(Component)]
pub struct DebugGridView {
    /// Whether or not to display the grid
    pub display_grid: bool,
    /// Whether or not to display the grid markers
    pub display_markers: bool,
    /// Color of the displayed grid.
    pub color: Color,
}
impl Default for DebugGridView {
    fn default() -> Self {
        Self {
            display_grid: true,
            display_markers: true,
            color: Default::default(),
        }
    }
}
impl DebugGridView {
    /// Creates a new [`DebugGridView`]
    pub fn new(display_grid: bool, display_markers: bool, color: Color) -> Self {
        Self {
            display_grid,
            display_markers,
            color,
        }
    }
}

/// System that uses [`Gizmos`] to render the debug grid every frame.
///
/// To be used with a [`bevy::prelude::Camera3d`]
pub fn draw_debug_grids_3d<T: CoordinateSystem>(
    mut gizmos: Gizmos,
    debug_grids: Query<(
        &Transform,
        &GridDefinition<T>,
        &DebugGridView,
        &DebugGridViewConfig3d,
    )>,
) {
    for (transform, grid, view, view_config) in debug_grids.iter() {
        if !view.display_grid {
            continue;
        }
        let start = &transform.translation;
        let end = Vec3 {
            x: start.x + (grid.size_x() as f32) * view_config.node_size.x,
            y: start.y + (grid.size_y() as f32) * view_config.node_size.y,
            z: start.z + (grid.size_z() as f32) * view_config.node_size.z,
        };
        for x in 0..=grid.size_x() {
            let mut points = Vec::with_capacity(4);
            let current_x = start.x + x as f32 * view_config.node_size.x;
            points.push(Vec3::new(current_x, start.y, start.z));
            points.push(Vec3::new(current_x, end.y, start.z));
            points.push(Vec3::new(current_x, end.y, end.z));
            points.push(Vec3::new(current_x, start.y, end.z));
            points.push(Vec3::new(current_x, start.y, start.z));
            gizmos.linestrip(points, view.color);
        }
        for y in 0..=grid.size_y() {
            let mut points = Vec::with_capacity(4);
            let current_y = start.y + y as f32 * view_config.node_size.y;
            points.push(Vec3::new(start.x, current_y, start.z));
            points.push(Vec3::new(end.x, current_y, start.z));
            points.push(Vec3::new(end.x, current_y, end.z));
            points.push(Vec3::new(start.x, current_y, end.z));
            points.push(Vec3::new(start.x, current_y, start.z));
            gizmos.linestrip(points, view.color);
        }
        for z in 0..=grid.size_z() {
            let mut points = Vec::with_capacity(4);
            let current_z = start.z + z as f32 * view_config.node_size.z;
            points.push(Vec3::new(start.x, start.y, current_z));
            points.push(Vec3::new(end.x, start.y, current_z));
            points.push(Vec3::new(end.x, end.y, current_z));
            points.push(Vec3::new(start.x, end.y, current_z));
            points.push(Vec3::new(start.x, start.y, current_z));
            gizmos.linestrip(points, view.color);
        }
    }
}

/// System that uses [`Gizmos`] to render the debug grid every frame.
///
/// To be used with a [`bevy::prelude::Camera2d`]
pub fn draw_debug_grids_2d<T: CoordinateSystem>(
    mut gizmos: Gizmos,
    debug_grids: Query<(
        &Transform,
        &GridDefinition<T>,
        &DebugGridView,
        &DebugGridViewConfig2d,
    )>,
) {
    for (transform, grid, view, view_config) in debug_grids.iter() {
        if !view.display_grid {
            continue;
        }
        let start = &transform.translation;
        let end = Vec2 {
            x: start.x + (grid.size_x() as f32) * view_config.node_size.x,
            y: start.y + (grid.size_y() as f32) * view_config.node_size.y,
        };
        for y in 0..=grid.size_y() {
            let current_y = start.y + y as f32 * view_config.node_size.y;
            let from = Vec2::new(start.x, current_y);
            let to = Vec2::new(end.x, current_y);
            gizmos.line_2d(from, to, view.color);
        }
        for x in 0..=grid.size_x() {
            let current_x = start.x + x as f32 * view_config.node_size.x;
            let from = Vec2::new(current_x, start.y);
            let to = Vec2::new(current_x, end.y);
            gizmos.line_2d(from, to, view.color);
        }
    }
}
