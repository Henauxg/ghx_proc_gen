use bevy::{
    ecs::{
        entity::Entity,
        event::{Event, EventReader},
        system::Query,
    },
    gizmos::gizmos::Gizmos,
    math::Vec3Swizzles,
    render::color::Color,
    transform::components::Transform,
};
use ghx_proc_gen::grid::GridPosition;

use super::{
    get_translation_from_grid_pos_2d, get_translation_from_grid_pos_3d,
    view::{DebugGridView, DebugGridViewConfig2d, DebugGridViewConfig3d},
    CoordinateSystem, Grid,
};

/// Event used to update markers on a [`DebugGridView`]
#[derive(Event)]
pub enum MarkerEvent {
    /// Send this event to create a new marker on a grid node
    Add {
        /// Color of the debug marker
        color: Color,
        /// Grid entity where the marker should be added
        grid_entity: Entity,
        /// Index of the grid node to mark
        node_index: usize,
    },
    /// Send this event to delete a marker on a grid node
    Remove {
        /// Grid entity from which the marker should be removed
        grid_entity: Entity,
        /// Index of the grid node to unmark
        node_index: usize,
    },
    /// Send this event to clear all markers on a grid
    Clear {
        /// Grid entity from which all markers should be removed
        grid_entity: Entity,
    },
    /// Send this event to clear all markers from all grids
    ClearAll,
}

#[derive(Clone)]
pub(crate) struct Marker {
    pub color: Color,
    pub pos: GridPosition,
}

/// This system reads [`MarkerEvent`] and update the [`DebugGridView`] components accordingly
///
/// Should be called after the systems that generate [`MarkerEvent`]
///
/// Called in the [`bevy::app::PostUpdate`] schedule by default, by the [`crate::grid::GridDebugPlugin`]
pub fn update_debug_markers<T: CoordinateSystem>(
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

/// This system draws 3d [`Gizmos`] on grids that have any markers on them and a [`DebugGridViewConfig3d`] component.
///
/// As with any gizmos, should be run once per frame for the rendering to persist.
pub fn draw_debug_markers_3d(
    mut gizmos: Gizmos,
    debug_grids: Query<(&Transform, &DebugGridView, &DebugGridViewConfig3d)>,
) {
    for (transform, view, view_config) in debug_grids.iter() {
        if !view.display_markers {
            continue;
        }
        for (_, marker) in view.markers.iter() {
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
) {
    for (transform, view, view_config) in debug_grids.iter() {
        if !view.display_markers {
            continue;
        }
        for (_, marker) in view.markers.iter() {
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
