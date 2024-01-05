use std::collections::HashMap;

use bevy::{
    asset::Assets,
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
    Grid, SharableDirectionSet,
};

#[derive(Bundle)]
pub struct DebugGridView3d {
    pub config: DebugGridViewConfig3d,
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

#[derive(Bundle)]
pub struct DebugGridView2d {
    pub config: DebugGridViewConfig2d,
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

#[derive(Component)]
pub struct DebugGridViewConfig3d {
    pub node_size: Vec3,
    pub color: Color,
}
impl Default for DebugGridViewConfig3d {
    fn default() -> Self {
        Self {
            node_size: Vec3::ONE,
            color: Default::default(),
        }
    }
}

#[derive(Component)]
pub struct DebugGridViewConfig2d {
    pub node_size: Vec2,
    pub color: Color,
}
impl Default for DebugGridViewConfig2d {
    fn default() -> Self {
        Self {
            node_size: Vec2::splat(32.),
            color: Default::default(),
        }
    }
}

#[derive(Component, Default)]
pub struct DebugGridMesh;

#[derive(Component)]
pub struct DebugGridView {
    pub(crate) markers: HashMap<usize, Marker>,
    pub display_grid: bool,
    pub display_markers: bool,
}
impl Default for DebugGridView {
    fn default() -> Self {
        Self {
            markers: Default::default(),
            display_grid: true,
            display_markers: true,
        }
    }
}
impl DebugGridView {
    pub fn new(display_grid: bool, display_markers: bool) -> Self {
        Self {
            markers: Default::default(),
            display_grid,
            display_markers,
        }
    }
}

pub fn spawn_debug_grids_3d<T: SharableDirectionSet>(
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
                    material: materials.add(LineMaterial {
                        color: view_config.color,
                    }),
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
            .insert(GridMeshMarker(debug_grid_mesh));
    }
}

#[derive(Component)]
pub struct GridMeshMarker(Entity);

pub fn update_debug_grid_mesh_visibility_3d(
    mut debug_grids: Query<(&GridMeshMarker, &DebugGridView), Changed<DebugGridView>>,
    mut grid_meshes: Query<&mut Visibility, With<DebugGridMesh>>,
) {
    for (grid_mesh_marker, view) in debug_grids.iter_mut() {
        match grid_meshes.get_mut(grid_mesh_marker.0) {
            Ok(mut mesh_visibility) => {
                *mesh_visibility = match view.display_grid {
                    true => Visibility::Visible,
                    false => Visibility::Hidden,
                }
            }
            Err(_) => (),
        }
    }
}

pub fn draw_debug_grids_2d<T: SharableDirectionSet>(
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
            gizmos.line_2d(from, to, view_config.color);
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
            gizmos.line_2d(from, to, view_config.color);
        }
    }
}
