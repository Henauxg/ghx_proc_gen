use std::collections::HashMap;

use bevy::{
    asset::{Assets, Handle},
    core::Name,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::{Added, Changed, With},
        system::{Commands, Query, ResMut},
    },
    gizmos::gizmos::Gizmos,
    hierarchy::BuildChildren,
    math::{Vec2, Vec3},
    pbr::MaterialMeshBundle,
    render::{color::Color, mesh::Mesh, view::Visibility},
    transform::components::Transform,
    utils::default,
};

use super::{
    lines::{LineList, LineMaterial},
    markers::Marker,
    Grid, SharableCoordSystem,
};

/// Add this bundle to a grid if you are using a 3d camera ([`bevy::prelude::Camera3d`]).
#[derive(Bundle)]
pub struct DebugGridView3d {
    /// 3d-specific configuration of the debug view
    pub config: DebugGridViewConfig3d,
    /// Debug view of the grid
    pub view: DebugGridView,
}
impl Default for DebugGridView3d {
    fn default() -> Self {
        Self {
            config: Default::default(),
            view: Default::default(),
        }
    }
}

/// Add this bundle to a grid if you are using a 2d camera ([`bevy::prelude::Camera2d`]).
#[derive(Bundle)]
pub struct DebugGridView2d {
    /// 2d-specific configuration of the debug view
    pub config: DebugGridViewConfig2d,
    /// Debug view of the grid
    pub view: DebugGridView,
}
impl Default for DebugGridView2d {
    fn default() -> Self {
        Self {
            config: Default::default(),
            view: Default::default(),
        }
    }
}

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

/// When an Entity with a [`Grid`] component has a [`DebugGridView3d`] bundle added to it. The plugin creates a child Entity with a 3d mesh representing the 3d grid.
///
/// This component is used to used to mark this child-entity to make it easy to change its [`Visibility`]
#[derive(Component, Default)]
pub struct DebugGridMesh;

/// When an Entity with a [`Grid`] component has a [`DebugGridView3d`] bundle added to it. The plugin creates a child Entity with a 3d mesh representing the 3d grid.
///
/// This component is used to used to mark the parent entity, and holds the child-entity id to make it easy to change the child entity [`Visibility`]
#[derive(Component)]
pub struct DebugGridMeshParent(Entity);

/// Component used on all debug grid to store markers and configuration.
///
/// Updating the component members will update the grid debug view directly
#[derive(Component)]
pub struct DebugGridView {
    pub(crate) markers: HashMap<usize, Marker>,
    /// Whether or not to display the grid
    pub display_grid: bool,
    /// Whether or not to display the grid markers
    pub display_markers: bool,
    /// Color of the displayed grid.
    ///
    /// Known limitation in 3d: updating the color will not update the grid
    pub color: Color,
}
impl Default for DebugGridView {
    fn default() -> Self {
        Self {
            markers: Default::default(),
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
            markers: Default::default(),
            display_grid,
            display_markers,
            color,
        }
    }
}

/// This system works on entities that have a [`Grid`] component and a [`DebugGridView3d`] bundle just added to them, it creates a child entity with its grid mesh and its own [`Visibility`]
///
/// To be used with a [`bevy::prelude::Camera3d`]
pub fn spawn_debug_grids_3d<T: SharableCoordSystem>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    debug_grids: Query<
        (Entity, &Grid<T>, &DebugGridView, &DebugGridViewConfig3d),
        Added<DebugGridViewConfig3d>,
    >,
) {
    // TODO Gizmos ? Performances may be worse than this mesh built once
    for (grid_entity, grid, view, view_config) in debug_grids.iter() {
        let mut lines = Vec::new();
        for y in 0..=grid.def.size_y() {
            let mut from = Vec3::new(0., y as f32, 0.);
            let mut to = Vec3::new(
                0.,
                y as f32,
                (grid.def.size_z() as f32) * view_config.node_size.z,
            );
            for x in 0..=grid.def.size_x() {
                from.x = view_config.node_size.x * x as f32;
                to.x = from.x;
                lines.push((from, to));
            }
            from = Vec3::new(0., y as f32, 0.);
            to = Vec3::new(
                grid.def.size_x() as f32 * view_config.node_size.x,
                y as f32,
                0.,
            );
            for z in 0..=grid.def.size_z() {
                from.z = view_config.node_size.z * z as f32;
                to.z = from.z;
                lines.push((from, to));
            }
        }
        for x in 0..=grid.def.size_x() {
            let mut from = Vec3::new(x as f32, 0., 0.);
            let mut to = Vec3::new(
                x as f32,
                grid.def.size_y() as f32 * view_config.node_size.y,
                0.,
            );
            for z in 0..=grid.def.size_z() {
                from.z = view_config.node_size.z * z as f32;
                to.z = from.z;
                lines.push((from, to));
            }
        }

        let debug_grid_mesh = commands
            .spawn((
                MaterialMeshBundle {
                    mesh: meshes.add(Mesh::from(LineList { lines })),
                    material: materials.add(LineMaterial { color: view.color }),
                    visibility: match view.display_grid {
                        true => Visibility::Visible,
                        false => Visibility::Hidden,
                    },
                    ..default()
                },
                Name::new("DebugGridMesh"),
                DebugGridMesh,
            ))
            .id();
        commands.entity(grid_entity).add_child(debug_grid_mesh);
        commands
            .entity(grid_entity)
            .insert(DebugGridMeshParent(debug_grid_mesh));
    }
}

/// System that detect the changes on the [`DebugGridView`] components and apply those changes to the underlying grid mesh (if any)
///
/// To be used with a [`bevy::prelude::Camera3d`]
pub fn update_debug_grid_mesh_visibility_3d(
    mut debug_grids: Query<(&DebugGridMeshParent, &DebugGridView), Changed<DebugGridView>>,
    mut grid_meshes: Query<(&mut Visibility, &Handle<LineMaterial>), With<DebugGridMesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    for (grid_mesh_marker, view) in debug_grids.iter_mut() {
        match grid_meshes.get_mut(grid_mesh_marker.0) {
            Ok((mut mesh_visibility, line_mat_handle)) => {
                *mesh_visibility = match view.display_grid {
                    true => Visibility::Visible,
                    false => Visibility::Hidden,
                };
                if let Some(line_mat) = materials.get_mut(line_mat_handle) {
                    line_mat.color = view.color;
                }
            }
            Err(_) => (),
        }
    }
}

/// System that uses [`Gizmos`] to render the debug grid every frame.
///
/// To be used with a [`bevy::prelude::Camera2d`]
pub fn draw_debug_grids_2d<T: SharableCoordSystem>(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &Grid<T>, &DebugGridView, &DebugGridViewConfig2d)>,
) {
    for (transform, grid, view, view_config) in debug_grids.iter() {
        if !view.display_grid {
            continue;
        }
        for y in 0..=grid.def.size_y() {
            let from = Vec2::new(
                transform.translation.x,
                transform.translation.y + y as f32 * view_config.node_size.y,
            );
            let to = Vec2::new(
                transform.translation.x + (grid.def.size_x() as f32) * view_config.node_size.x,
                transform.translation.y + y as f32 * view_config.node_size.y,
            );
            gizmos.line_2d(from, to, view.color);
        }
        for x in 0..=grid.def.size_x() {
            let from = Vec2::new(
                transform.translation.x + x as f32 * view_config.node_size.x,
                transform.translation.y,
            );
            let to = Vec2::new(
                transform.translation.x + x as f32 * view_config.node_size.x,
                transform.translation.y + (grid.def.size_y() as f32) * view_config.node_size.y,
            );
            gizmos.line_2d(from, to, view.color);
        }
    }
}
