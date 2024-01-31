use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::With,
        system::{Commands, Query},
    },
    gizmos::gizmos::Gizmos,
    hierarchy::{BuildChildren, Parent},
    math::Vec3Swizzles,
    render::color::Color,
    transform::components::Transform,
};
use ghx_proc_gen::grid::{GridDefinition, GridPosition, NodeIndex};

use super::{
    get_translation_from_grid_pos_2d, get_translation_from_grid_pos_3d,
    view::{DebugGridView, DebugGridViewConfig2d, DebugGridViewConfig3d},
    CoordinateSystem,
};

/// Event used to despawn markers on a [`DebugGridView`]
#[derive(Event)]
pub enum MarkerDespawnEvent {
    /// Send this event to delete a marker on a grid node
    Remove {
        /// Marker `Entity`
        marker_entity: Entity,
    },
    /// Send this event to clear all markers on a grid
    Clear {
        /// Grid entity from which all markers should be removed
        grid_entity: Entity,
    },
    /// Send this event to clear all markers from all grids
    ClearAll,
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

/// Helper to spwan a [`Marker`] `Entity` that will be displayed by the [`super::GridDebugPlugin`]
pub fn spawn_marker<C: CoordinateSystem>(
    commands: &mut Commands,
    grid: &GridDefinition<C>,
    grid_entity: Entity,
    color: Color,
    node_index: NodeIndex,
) -> Entity {
    let marker_entity = commands
        .spawn(GridMarker::new(color, grid.pos_from_index(node_index)))
        .id();
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
            MarkerDespawnEvent::Remove { marker_entity } => {
                if let Ok(_) = markers.get(*marker_entity) {
                    commands.entity(*marker_entity).despawn();
                }
            }
            MarkerDespawnEvent::Clear { grid_entity } => {
                for (parent_grid, marker_entity) in markers.iter() {
                    if parent_grid.get() == *grid_entity {
                        if let Ok(_) = markers.get(marker_entity) {
                            commands.entity(marker_entity).despawn();
                        }
                    }
                }
            }
            MarkerDespawnEvent::ClearAll => {
                for (_parent_grid, marker_entity) in markers.iter() {
                    if let Ok(_) = markers.get(marker_entity) {
                        commands.entity(marker_entity).despawn();
                    }
                }
            }
        }
    }
}

/// This system draws 3d [`Gizmos`] on grids that have any markers on them and a [`DebugGridViewConfig3d`] component.
///
/// As with any gizmos, should be run once per frame for the rendering to persist.
pub fn draw_debug_markers_3d(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &DebugGridView, &DebugGridViewConfig3d)>,
    markers: Query<(&Parent, &GridMarker)>,
) {
    for (parent_grid, marker) in markers.iter() {
        if let Ok((transform, view, view_config)) = debug_grids.get(parent_grid.get()) {
            if !view.display_markers {
                continue;
            }
            let giz_pos = transform.translation
                + get_translation_from_grid_pos_3d(&marker.pos, &view_config.node_size);
            gizmos.cuboid(
                // Scale a bit so that it is not on the grid outlines.
                Transform::from_translation(giz_pos).with_scale(view_config.node_size * 1.05),
                marker.color,
            );
        }
    }
}

/// This system draws 2d [`Gizmos`] on grids that have any markers on them and a [`DebugGridViewConfig2d`] component.
///
/// As with any gizmos, should be run once per frame for the rendering to persist.
pub fn draw_debug_markers_2d(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &DebugGridView, &DebugGridViewConfig2d)>,
    markers: Query<(&Parent, &GridMarker)>,
) {
    for (parent_grid, marker) in markers.iter() {
        if let Ok((transform, view, view_config)) = debug_grids.get(parent_grid.get()) {
            if !view.display_markers {
                continue;
            }
            let giz_pos = transform.translation.xy()
                + get_translation_from_grid_pos_2d(&marker.pos, &view_config.node_size);
            gizmos.rect_2d(
                giz_pos,
                0.,
                // Scale a bit so that it is not on the grid outlines.
                view_config.node_size * 1.05,
                marker.color,
            );
        }
    }
}
