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

use bevy_mod_picking::prelude::{Down, ListenerInput, On, Over, Pointer};
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
pub struct OverCursor(pub GridCursor);
impl GridCursorContainer for OverCursor {
    fn new(cursor: GridCursor) -> Self {
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
    mut selection_cursor_text: Query<&mut Text, With<CursorsPanelText>>,
    mut updated_cursors: Query<
        (&OverCursorInfo, &OverCursor, &ActiveGridCursor),
        Changed<OverCursorInfo>,
    >,
) {
    if let Ok((cursor_info, cursor, _active)) = updated_cursors.get_single() {
        for mut text in &mut selection_cursor_text {
            text.sections[OVER_CURSOR_SECTION_INDEX].value =
                format!("Hovered:\n{}", cursor_info_to_string(cursor, cursor_info));
        }
    }
}

pub fn picking_update_cursors_position<
    C: CoordinateSystem,
    GCS: GridCursorMarkerSettings,
    GC: Component + std::ops::DerefMut<Target = GridCursor>,
    E: Event + std::ops::DerefMut<Target = Entity>,
>(
    mut events: EventReader<E>,
    mut commands: Commands,
    cursor_marker_settings: Res<GCS>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<(&SpawnedNode, &Parent)>,
    mut cursors: Query<(&mut GC, &GridDefinition<C>)>,
) {
    for event in events.read().last() {
        if let Ok((node, node_parent)) = nodes.get_mut(**event) {
            let parent_entity = node_parent.get();
            if let Ok((mut cursor, grid)) = cursors.get_mut(parent_entity) {
                if cursor.node_index != node.0 {
                    cursor.node_index = node.0;
                    cursor.position = grid.pos_from_index(node.0);

                    if let Some(previous_cursor_entity) = cursor.marker {
                        marker_events.send(MarkerDespawnEvent::Remove {
                            marker_entity: previous_cursor_entity,
                        });
                    }
                    let marker_entity = commands
                        .spawn(GridMarker::new(
                            cursor_marker_settings.color(),
                            cursor.position.clone(),
                        ))
                        .id();
                    commands.entity(parent_entity).add_child(marker_entity);
                    cursor.marker = Some(marker_entity);
                }
            }
        }
    }
}
