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
    Grid, SharableDirectionSet,
};

#[derive(Clone)]
pub struct Marker {
    pub color: Color,
    pub pos: GridPosition,
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
