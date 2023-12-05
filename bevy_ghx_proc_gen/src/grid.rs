use bevy::{
    asset::Assets,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query, ResMut},
    },
    hierarchy::BuildChildren,
    math::Vec3,
    pbr::MaterialMeshBundle,
    render::{color::Color, mesh::Mesh},
    utils::default,
};
use ghx_proc_gen::grid::{direction::DirectionSet, GridDefinition};

use crate::lines::{LineList, LineMaterial};

#[derive(Component)]
pub struct DebugGridViewConfig {
    pub node_size: Vec3,
    pub color: Color,
    // grid_size: Vec3,
}

#[derive(Component)]
pub struct DebugGridView;

#[derive(Component)]
pub struct Grid<T: DirectionSet + Clone + Sync + Send> {
    pub def: GridDefinition<T>,
}

pub fn spawn_debug_grids<T: DirectionSet + Clone + Sync + Send + 'static>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    debug_grids: Query<(Entity, &Grid<T>, &DebugGridViewConfig), Added<DebugGridViewConfig>>,
) {
    for (grid_entity, grid, view_config) in debug_grids.iter() {
        let mut lines = vec![];
        for y in 0..=grid.def.size_y() {
            let mut from = Vec3::new(0., y as f32, 0.);
            let mut to = Vec3::new(
                0.,
                y as f32,
                -(grid.def.size_z() as f32) * view_config.node_size.z,
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
                from.z = -(view_config.node_size.z * z as f32);
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
                from.z = -(view_config.node_size.z * z as f32);
                to.z = from.z;
                lines.push((from, to));
            }
        }

        let debug_grid = commands
            .spawn((
                MaterialMeshBundle {
                    mesh: meshes.add(Mesh::from(LineList { lines })),
                    material: materials.add(LineMaterial {
                        color: view_config.color,
                    }),
                    ..default()
                },
                Name::new("DebugView"),
                DebugGridView,
            ))
            .id();
        commands.entity(grid_entity).add_child(debug_grid);
    }
}
