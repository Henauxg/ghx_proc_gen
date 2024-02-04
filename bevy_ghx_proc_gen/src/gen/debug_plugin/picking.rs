use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::{Added, Changed, With},
        system::{Commands, Query, Res, Resource},
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
use ghx_proc_gen::grid::{direction::CoordinateSystem, GridDefinition};

use crate::{
    gen::SpawnedNode,
    grid::markers::{GridMarker, MarkerDespawnEvent},
};

use super::cursor::{
    cursor_info_to_string, ActiveGridCursor, CursorsPanelText, GridCursor, GridCursorContainer,
    GridCursorInfo, GridCursorInfoContainer, GridCursorMarkerSettings, GridCursorOverlay,
    OVER_CURSOR_SECTION_INDEX,
};

#[derive(Resource)]
pub struct OverCursorMarkerSettings(pub Color);
impl Default for OverCursorMarkerSettings {
    fn default() -> Self {
        Self(Color::rgb(0.85, 0.85, 0.73))
    }
}
impl GridCursorMarkerSettings for OverCursorMarkerSettings {
    fn color(&self) -> Color {
        self.0
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct OverCursor(pub Option<GridCursor>);
impl GridCursorContainer for OverCursor {
    fn new(cursor: Option<GridCursor>) -> Self {
        Self(cursor)
    }
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct OverCursorInfo(pub GridCursorInfo);
impl GridCursorInfoContainer for OverCursorInfo {
    fn new(cursor_info: GridCursorInfo) -> Self {
        Self(cursor_info)
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct OverCursorOverlay(pub Entity);
impl GridCursorOverlay for OverCursorOverlay {
    fn new(grid_entity: Entity) -> Self {
        Self(grid_entity)
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
    mut updated_cursors: Query<
        (&OverCursorInfo, &OverCursor, &ActiveGridCursor),
        Changed<OverCursorInfo>,
    >,
) {
    if let Ok((cursor_info, cursor, _active)) = updated_cursors.get_single() {
        for mut text in &mut cursors_panel_text {
            let ui_text = &mut text.sections[OVER_CURSOR_SECTION_INDEX].value;
            match cursor.as_ref() {
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
    GCS: GridCursorMarkerSettings,
    GC: GridCursorContainer,
    E: Event + std::ops::DerefMut<Target = Entity>,
>(
    mut commands: Commands,
    cursor_marker_settings: Res<GCS>,
    mut events: EventReader<E>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<(&SpawnedNode, &Parent)>,
    mut cursors: Query<(&mut GC, &GridDefinition<C>)>,
) {
    if let Some(event) = events.read().last() {
        if let Ok((node, node_parent)) = nodes.get_mut(**event) {
            let parent_entity = node_parent.get();
            if let Ok((mut cursor, grid)) = cursors.get_mut(parent_entity) {
                let update_cursor = match (&*cursor).as_ref() {
                    Some(grid_cursor) => {
                        if grid_cursor.node_index != node.0 {
                            marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                            true
                        } else {
                            false
                        }
                    }
                    None => true,
                };

                if update_cursor {
                    let position = grid.pos_from_index(node.0);
                    let marker = commands
                        .spawn(GridMarker::new(cursor_marker_settings.color(), position))
                        .id();
                    commands.entity(parent_entity).add_child(marker);
                    **cursor = Some(GridCursor {
                        node_index: node.0,
                        position,
                        marker,
                    });
                }
            }
        }
    }
}

pub fn picking_remove_previous_over_cursor<C: CoordinateSystem>(
    mut out_events: EventReader<NodeOutEvent>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<&Parent, With<SpawnedNode>>,
    mut cursors: Query<&mut OverCursor>,
) {
    if let Some(event) = out_events.read().last() {
        if let Ok(node_parent) = nodes.get_mut(**event) {
            if let Ok(mut cursor) = cursors.get_mut(node_parent.get()) {
                if let Some(grid_cursor) = &cursor.0 {
                    marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                    **cursor = None;
                }
            }
        }
    }
}
