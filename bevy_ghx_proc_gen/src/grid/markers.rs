use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{With, Without},
        system::{Commands, Query},
    },
    gizmos::gizmos::Gizmos,
    hierarchy::{BuildChildren, DespawnRecursiveExt, Parent},
    math::Vec3Swizzles,
    prelude::SpatialBundle,
    render::color::Color,
    transform::components::{GlobalTransform, Transform},
};
use ghx_proc_gen::grid::GridPosition;

use super::{
    get_translation_from_grid_pos_3d,
    view::{DebugGridView, DebugGridView2d, DebugGridView3d},
};

/// Event used to despawn markers on a [`DebugGridView`]
#[derive(Event)]
pub enum MarkerDespawnEvent {
    /// Send this event to delete a marker Entity
    Marker(Entity),
    /// Send this event to clear all markers on a grid Entity
    Grid(Entity),
    /// Send this event to clear all markers from all grids
    All,
}

/// Marker to be displayed on a grid
#[derive(Component)]
pub struct GridMarker {
    /// Color of the marker gizmo
    pub color: Color,
    /// Grid position of the marker
    pub pos: GridPosition,
}
impl GridMarker {
    /// Helper to construct a marker
    pub fn new(color: Color, pos: GridPosition) -> Self {
        Self { color, pos }
    }
}

/// Helper to spwan a [`GridMarker`] `Entity` that will be displayed by the [`super::GridDebugPlugin`]
pub fn spawn_marker(
    commands: &mut Commands,
    grid_entity: Entity,
    color: Color,
    pos: GridPosition,
) -> Entity {
    let marker_entity = commands.spawn(GridMarker { color, pos }).id();
    commands.entity(grid_entity).add_child(marker_entity);
    marker_entity
}

/// This system reads [`MarkerDespawnEvent`] and despawn markers entities accordingly. Tries to check for existence before despawning them.
///
/// Should be called after the systems that generate [`MarkerDespawnEvent`]
///
/// Called in the [`bevy::app::PostUpdate`] schedule by default, by the [`crate::grid::GridDebugPlugin`]
pub fn update_debug_markers(
    mut commands: Commands,
    mut marker_events: EventReader<MarkerDespawnEvent>,
    markers: Query<(&Parent, Entity), With<GridMarker>>,
) {
    for marker_event in marker_events.read() {
        match marker_event {
            MarkerDespawnEvent::Marker(marker_entity) => {
                if let Ok(_) = markers.get(*marker_entity) {
                    commands.entity(*marker_entity).despawn_recursive();
                }
            }
            MarkerDespawnEvent::Grid(grid_entity) => {
                for (parent_grid, marker_entity) in markers.iter() {
                    if parent_grid.get() == *grid_entity {
                        if let Ok(_) = markers.get(marker_entity) {
                            commands.entity(marker_entity).despawn_recursive();
                        }
                    }
                }
            }
            MarkerDespawnEvent::All => {
                for (_parent_grid, marker_entity) in markers.iter() {
                    if let Ok(_) = markers.get(marker_entity) {
                        commands.entity(marker_entity).despawn_recursive();
                    }
                }
            }
        }
    }
}

pub fn insert_transform_on_new_markers(
    mut commands: Commands,
    debug_grid_views: Query<&DebugGridView>,
    mut new_markers: Query<(&Parent, Entity, &GridMarker), Without<Transform>>,
) {
    for (grid_entity, marker_entity, marker) in &mut new_markers {
        if let Ok(view) = debug_grid_views.get(grid_entity.get()) {
            let marker_translation = get_translation_from_grid_pos_3d(&marker.pos, &view.node_size);
            commands
                .entity(marker_entity)
                .try_insert(SpatialBundle::from_transform(Transform::from_translation(
                    marker_translation,
                )));
        }
    }
}

/// This system draws 3d [`Gizmos`] on grids that have any markers on them and a [`DebugGridView3d`] component.
///
/// As with any gizmos, should be run once per frame for the rendering to persist.
pub fn draw_debug_markers_3d(
    mut gizmos: Gizmos,
    debug_grid_views: Query<&DebugGridView, With<DebugGridView3d>>,
    markers: Query<(&Parent, &GlobalTransform, &GridMarker)>,
) {
    for (parent_grid, global_transform, marker) in markers.iter() {
        if let Ok(view) = debug_grid_views.get(parent_grid.get()) {
            if !view.display_markers {
                continue;
            }
            gizmos.cuboid(
                // Scale a bit so that it is not on the grid outlines.
                Transform::from(*global_transform).with_scale(view.node_size * 1.05),
                marker.color,
            );
        }
    }
}

/// This system draws 2d [`Gizmos`] on grids that have any markers on them and a [`DebugGridView2d`] component.
///
/// As with any gizmos, should be run once per frame for the rendering to persist.
pub fn draw_debug_markers_2d(
    mut gizmos: Gizmos,
    debug_grid_views: Query<&DebugGridView, With<DebugGridView2d>>,
    markers: Query<(&Parent, &GlobalTransform, &GridMarker)>,
) {
    for (parent_grid, global_transform, marker) in markers.iter() {
        if let Ok(view) = debug_grid_views.get(parent_grid.get()) {
            if !view.display_markers {
                continue;
            }
            let node_size = view.node_size.xy();
            let (_scale, rot, translation) = global_transform.to_scale_rotation_translation();
            gizmos.rect_2d(
                translation.xy(),
                rot.to_axis_angle().1,
                // Scale a bit so that it is not on the grid outlines.
                node_size * 1.05,
                marker.color,
            );
        }
    }
}
