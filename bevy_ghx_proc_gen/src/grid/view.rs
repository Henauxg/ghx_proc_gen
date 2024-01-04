use std::collections::HashMap;

use bevy::{
    asset::Assets,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query, ResMut},
    },
    gizmos::gizmos::Gizmos,
    hierarchy::BuildChildren,
    math::{Vec2, Vec3},
    pbr::MaterialMeshBundle,
    render::mesh::Mesh,
    transform::components::Transform,
    utils::default,
};

use super::{
    lines::{LineList, LineMaterial},
    markers::Marker,
    DebugGridViewConfig2d, DebugGridViewConfig3d, Grid, SharableDirectionSet,
};

#[derive(Component, Default)]
pub struct DebugGridMesh;

#[derive(Component)]
pub struct DebugGridView {
    pub(crate) markers: HashMap<usize, Marker>,
}

impl Default for DebugGridView {
    fn default() -> Self {
        Self {
            markers: Default::default(),
        }
    }
}

pub fn spawn_debug_grids_3d<T: SharableDirectionSet>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    debug_grids: Query<(Entity, &Grid<T>, &DebugGridViewConfig3d), Added<DebugGridViewConfig3d>>,
) {
    // TODO Gizmos ? Performances may be worse than this mesh built once
    for (grid_entity, grid, view_config) in debug_grids.iter() {
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
                    ..default()
                },
                Name::new("DebugGridMesh"),
                DebugGridMesh,
            ))
            .id();
        commands.entity(grid_entity).add_child(debug_grid_mesh);
        commands
            .entity(grid_entity)
            .insert(DebugGridView::default());
    }
}

pub fn spawn_debug_grids_2d<T: SharableDirectionSet>(
    mut commands: Commands,
    debug_grids: Query<Entity, Added<DebugGridViewConfig2d>>,
) {
    for grid_entity in debug_grids.iter() {
        commands
            .entity(grid_entity)
            .insert(DebugGridView::default());
    }
}

pub fn draw_debug_grids_2d<T: SharableDirectionSet>(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &Grid<T>, &DebugGridViewConfig2d)>,
) {
    for (transform, grid, view_config) in debug_grids.iter() {
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
