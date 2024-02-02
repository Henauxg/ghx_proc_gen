use bevy::{
    ecs::{component::Component, query::With, system::Query},
    gizmos::gizmos::Gizmos,
    math::{Vec2, Vec3, Vec3Swizzles},
    render::color::Color,
    transform::components::Transform,
};
use ghx_proc_gen::grid::GridDefinition;

use super::CoordinateSystem;

/// 3d-specific ([`bevy::prelude::Camera3d`]) component-marker of a grid debug view
#[derive(Component, Default)]
pub struct DebugGridView3d;

/// 2d-specific ([`bevy::prelude::Camera2d`]) component-marker of a grid debug view
#[derive(Component, Default)]
pub struct DebugGridView2d;

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
    /// Size of a grid node in world units on all 3 axis. Defaults to [`Vec3::ONE`]
    pub node_size: Vec3,
}
impl Default for DebugGridView {
    fn default() -> Self {
        Self {
            display_grid: true,
            display_markers: true,
            color: Default::default(),
            node_size: Vec3::ONE,
        }
    }
}
impl DebugGridView {
    /// Creates a new [`DebugGridView`]
    pub fn new(display_grid: bool, display_markers: bool, color: Color, node_size: Vec3) -> Self {
        Self {
            display_grid,
            display_markers,
            color,
            node_size,
        }
    }
}

/// System that uses [`Gizmos`] to render the debug grid every frame.
///
/// To be used with a [`bevy::prelude::Camera3d`]
pub fn draw_debug_grids_3d<T: CoordinateSystem>(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &GridDefinition<T>, &DebugGridView), With<DebugGridView3d>>,
) {
    for (transform, grid, view) in debug_grids.iter() {
        if !view.display_grid {
            continue;
        }
        let end = Vec3 {
            x: (grid.size_x() as f32) * view.node_size.x,
            y: (grid.size_y() as f32) * view.node_size.y,
            z: (grid.size_z() as f32) * view.node_size.z,
        };
        for x in 0..=grid.size_x() {
            let current_x = x as f32 * view.node_size.x;
            let points = vec![
                transform.transform_point(Vec3::new(current_x, 0., 0.)),
                transform.transform_point(Vec3::new(current_x, end.y, 0.)),
                transform.transform_point(Vec3::new(current_x, end.y, end.z)),
                transform.transform_point(Vec3::new(current_x, 0., end.z)),
                transform.transform_point(Vec3::new(current_x, 0., 0.)),
            ];
            gizmos.linestrip(points, view.color);
        }
        for y in 0..=grid.size_y() {
            let current_y = y as f32 * view.node_size.y;
            let points = vec![
                transform.transform_point(Vec3::new(0., current_y, 0.)),
                transform.transform_point(Vec3::new(end.x, current_y, 0.)),
                transform.transform_point(Vec3::new(end.x, current_y, end.z)),
                transform.transform_point(Vec3::new(0., current_y, end.z)),
                transform.transform_point(Vec3::new(0., current_y, 0.)),
            ];
            gizmos.linestrip(points, view.color);
        }
        for z in 0..=grid.size_z() {
            let current_z = z as f32 * view.node_size.z;
            let points = vec![
                transform.transform_point(Vec3::new(0., 0., current_z)),
                transform.transform_point(Vec3::new(end.x, 0., current_z)),
                transform.transform_point(Vec3::new(end.x, end.y, current_z)),
                transform.transform_point(Vec3::new(0., end.y, current_z)),
                transform.transform_point(Vec3::new(0., 0., current_z)),
            ];
            gizmos.linestrip(points, view.color);
        }
    }
}

/// System that uses [`Gizmos`] to render the debug grid every frame.
///
/// To be used with a [`bevy::prelude::Camera2d`]
pub fn draw_debug_grids_2d<T: CoordinateSystem>(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &GridDefinition<T>, &DebugGridView), With<DebugGridView2d>>,
) {
    for (transform, grid, view) in debug_grids.iter() {
        if !view.display_grid {
            continue;
        }
        let end = Vec2 {
            x: (grid.size_x() as f32) * view.node_size.x,
            y: (grid.size_y() as f32) * view.node_size.y,
        };
        for y in 0..=grid.size_y() {
            let current_y = y as f32 * view.node_size.y;
            let from = transform.transform_point(Vec3::new(0., current_y, 0.));
            let to = transform.transform_point(Vec3::new(end.x, current_y, 0.));
            gizmos.line_2d(from.xy(), to.xy(), view.color);
        }
        for x in 0..=grid.size_x() {
            let current_x = x as f32 * view.node_size.x;
            let from = transform.transform_point(Vec3::new(current_x, 0., 0.));
            let to = transform.transform_point(Vec3::new(current_x, end.y, 0.));
            gizmos.line_2d(from.xy(), to.xy(), view.color);
        }
    }
}
