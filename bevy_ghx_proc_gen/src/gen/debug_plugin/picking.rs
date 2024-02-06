use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::{Added, Changed, With},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::{BuildChildren, Parent},
    prelude::{Deref, DerefMut},
    render::color::Color,
    text::Text,
};

use bevy_mod_picking::{
    events::Out,
    prelude::{Down, ListenerInput, On, Over, Pointer},
};
use ghx_proc_gen::{
    generator::Generator,
    grid::{direction::CoordinateSystem, GridDefinition},
};

use crate::{
    gen::SpawnedNode,
    grid::markers::{GridMarker, MarkerDespawnEvent},
};

use super::{
    cursor::{
        cursor_info_to_string, Cursor, CursorBehavior, CursorInfo, CursorMarkerSettings,
        CursorsPanelText, GridCursor, OVER_CURSOR_SECTION_INDEX,
    },
    generation::ActiveGeneration,
};

#[derive(Resource)]
pub struct OverCursorMarkerSettings(pub Color);
impl Default for OverCursorMarkerSettings {
    fn default() -> Self {
        Self(Color::rgb(0.85, 0.85, 0.73))
    }
}
impl CursorMarkerSettings for OverCursorMarkerSettings {
    fn color(&self) -> Color {
        self.0
    }
}

#[derive(Component, Debug)]
pub struct OverCursor;
impl CursorBehavior for OverCursor {
    fn new() -> Self {
        Self
    }
    fn updates_active_gen() -> bool {
        false
    }
}

#[derive(Event, Deref, DerefMut)]
pub struct NodeOverEvent(pub Entity);
impl From<ListenerInput<Pointer<Over>>> for NodeOverEvent {
    fn from(event: ListenerInput<Pointer<Over>>) -> Self {
        NodeOverEvent(event.listener())
    }
}

#[derive(Event, Deref, DerefMut)]
pub struct NodeOutEvent(pub Entity);
impl From<ListenerInput<Pointer<Out>>> for NodeOutEvent {
    fn from(event: ListenerInput<Pointer<Out>>) -> Self {
        NodeOutEvent(event.listener())
    }
}

#[derive(Event, Deref, DerefMut)]
pub struct NodeSelectedEvent(pub Entity);

pub fn insert_grid_cursor_picking_handlers_to_spawned_nodes<C: CoordinateSystem>(
    mut commands: Commands,
    spawned_nodes: Query<Entity, Added<SpawnedNode>>,
) {
    use bevy_mod_picking::{pointer::PointerButton, prelude::ListenerMut};

    for node in spawned_nodes.iter() {
        commands
            .entity(node)
            .try_insert(On::<Pointer<Over>>::send_event::<NodeOverEvent>());
        commands
            .entity(node)
            .try_insert(On::<Pointer<Out>>::send_event::<NodeOutEvent>());
        commands.entity(node).try_insert(On::<Pointer<Down>>::run(
            move |event: ListenerMut<Pointer<Down>>,
                  mut selection_events: EventWriter<NodeSelectedEvent>| {
                if event.button == PointerButton::Primary {
                    selection_events.send(NodeSelectedEvent(event.listener()));
                }
            },
        ));
    }
}

pub fn update_over_cursor_panel_text(
    mut cursors_panel_text: Query<&mut Text, With<CursorsPanelText>>,
    mut updated_cursors: Query<(&CursorInfo, &Cursor), (Changed<CursorInfo>, With<OverCursor>)>,
) {
    if let Ok((cursor_info, cursor)) = updated_cursors.get_single() {
        for mut text in &mut cursors_panel_text {
            let ui_text = &mut text.sections[OVER_CURSOR_SECTION_INDEX].value;
            match &cursor.0 {
                Some(grid_cursor) => {
                    *ui_text = format!(
                        "Hovered:\n{}",
                        cursor_info_to_string(grid_cursor, cursor_info)
                    );
                }
                None => ui_text.clear(),
            }
        }
    }
}

pub fn picking_update_cursors_position<
    C: CoordinateSystem,
    CS: CursorMarkerSettings,
    CB: CursorBehavior,
    PE: Event + std::ops::DerefMut<Target = Entity>,
>(
    mut commands: Commands,
    cursor_marker_settings: Res<CS>,
    mut active_generation: ResMut<ActiveGeneration>,
    mut events: EventReader<PE>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<(&SpawnedNode, &Parent)>,
    mut cursor: Query<&mut Cursor, With<CB>>,
    generations: Query<(Entity, &GridDefinition<C>), With<Generator<C>>>,
) {
    if let Some(event) = events.read().last() {
        let Ok(mut cursor) = cursor.get_single_mut() else {
            return;
        };

        if let Ok((node, node_parent)) = nodes.get_mut(*event.deref()) {
            let picked_grid_entity = node_parent.get();
            let update_cursor = match &cursor.0 {
                Some(grid_cursor) => {
                    if (grid_cursor.grid != picked_grid_entity)
                        || (grid_cursor.node_index != node.0)
                    {
                        marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                        true
                    } else {
                        false
                    }
                }
                None => true,
            };

            if update_cursor {
                let Ok((gen_entity, grid)) = generations.get(picked_grid_entity) else {
                    return;
                };

                if CB::updates_active_gen() {
                    active_generation.0 = Some(gen_entity);
                }
                let position = grid.pos_from_index(node.0);
                let marker = commands
                    .spawn(GridMarker::new(cursor_marker_settings.color(), position))
                    .id();
                commands.entity(picked_grid_entity).add_child(marker);
                cursor.0 = Some(GridCursor {
                    grid: picked_grid_entity,
                    node_index: node.0,
                    position,
                    marker,
                });
            }
        }
    }
}

pub fn picking_remove_previous_over_cursor<C: CoordinateSystem>(
    mut out_events: EventReader<NodeOutEvent>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<&SpawnedNode>,
    mut over_cursor: Query<&mut Cursor, With<OverCursor>>,
) {
    if let Some(event) = out_events.read().last() {
        let Ok(mut cursor) = over_cursor.get_single_mut() else {
            return;
        };
        let Some(grid_cursor) = &cursor.0 else {
            return;
        };
        if let Ok(node) = nodes.get_mut(**event) {
            if grid_cursor.node_index == node.0 {
                marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                cursor.0 = None;
            }
        }
    }
}
