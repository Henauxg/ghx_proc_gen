use std::collections::HashMap;

use bevy::{
    asset::Assets,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::Added,
        system::{Commands, Query, ResMut},
    },
    gizmos::gizmos::Gizmos,
    hierarchy::BuildChildren,
    math::Vec3,
    pbr::MaterialMeshBundle,
    render::{color::Color, mesh::Mesh},
    transform::components::Transform,
    utils::default,
};
use ghx_proc_gen::grid::{direction::DirectionSet, GridDefinition, GridPosition};

use crate::lines::{LineList, LineMaterial};

#[derive(Component)]
pub struct DebugGridViewConfig {
    pub node_size: Vec3,
    pub color: Color,
    // grid_size: Vec3,
}

#[derive(Clone)]
pub struct Marker {
    pub color: Color,
    pub pos: GridPosition,
}

#[derive(Component, Default)]
pub struct DebugGridMesh;
impl DebugGridMesh {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Component, Default)]
pub struct DebugGridView {
    node_size: Vec3,
    markers: HashMap<usize, Marker>,
}
impl DebugGridView {
    fn new(node_size: Vec3) -> Self {
        Self {
            node_size,
            markers: HashMap::new(),
        }
    }
}

pub trait SharableDirectionSet: DirectionSet + Clone + Sync + Send + 'static {}
impl<T: DirectionSet + Clone + Sync + Send + 'static> SharableDirectionSet for T {}

#[derive(Component)]
pub struct Grid<T: SharableDirectionSet> {
    pub def: GridDefinition<T>,
}

#[derive(Event)]
pub enum MarkerEvent {
    Add {
        color: Color,
        grid_entity: Entity,
        node_index: usize,
    },
    Remove {
        grid_entity: Entity,
        node_index: usize,
    },
    Clear {
        grid_entity: Entity,
    },
    ClearAll,
}

pub fn update_debug_markers<T: SharableDirectionSet>(
    mut marker_events: EventReader<MarkerEvent>,
    mut debug_grids: Query<(&Grid<T>, &mut DebugGridView)>,
) {
    for marker_event in marker_events.read() {
        match marker_event {
            MarkerEvent::Add {
                color,
                grid_entity,
                node_index,
            } => {
                if let Ok((grid, mut debug_grid)) = debug_grids.get_mut(*grid_entity) {
                    debug_grid.markers.insert(
                        *node_index,
                        Marker {
                            color: *color,
                            pos: grid.def.get_position(*node_index),
                        },
                    );
                }
            }
            MarkerEvent::Remove {
                grid_entity,
                node_index,
            } => {
                if let Ok((_grid, mut debug_grid)) = debug_grids.get_mut(*grid_entity) {
                    debug_grid.markers.remove(node_index);
                }
            }
            MarkerEvent::Clear { grid_entity } => {
                if let Ok((_grid, mut debug_grid)) = debug_grids.get_mut(*grid_entity) {
                    debug_grid.markers.clear();
                }
            }
            MarkerEvent::ClearAll => {
                for (_grid, mut debug_grid) in debug_grids.iter_mut() {
                    debug_grid.markers.clear();
                }
            }
        }
    }
}

pub fn draw_debug_markers(mut gizmos: Gizmos, debug_grids: Query<(&Transform, &DebugGridView)>) {
    for (transform, debug_grid) in debug_grids.iter() {
        for (_, marker) in debug_grid.markers.iter() {
            let giz_pos = transform.translation
                + get_translation_from_grid_pos(&marker.pos, &debug_grid.node_size);
            gizmos.cuboid(
                Transform::from_translation(giz_pos).with_scale(debug_grid.node_size),
                marker.color,
            );
        }
    }
}

pub fn spawn_debug_grids<T: SharableDirectionSet>(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    debug_grids: Query<(Entity, &Grid<T>, &DebugGridViewConfig), Added<DebugGridViewConfig>>,
) {
    // TODO Gizmos ? :)
    for (grid_entity, grid, view_config) in debug_grids.iter() {
        let mut lines = vec![];
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
                Name::new("DebugGridView"),
                DebugGridMesh::new(),
            ))
            .id();
        commands.entity(grid_entity).add_child(debug_grid_mesh);
        commands
            .entity(grid_entity)
            .insert(DebugGridView::new(view_config.node_size));
    }
}

#[inline]
pub fn get_translation_from_grid_pos(grid_pos: &GridPosition, node_size: &Vec3) -> Vec3 {
    Vec3 {
        x: (grid_pos.x as f32 + 0.5) * node_size.x,
        y: (grid_pos.y as f32 + 0.5) * node_size.y,
        z: (grid_pos.z as f32 + 0.5) * node_size.z,
    }
}
