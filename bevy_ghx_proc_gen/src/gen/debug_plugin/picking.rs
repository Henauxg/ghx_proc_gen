use bevy::{
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::{Added, Changed, With, Without},
        system::{Commands, Local, Query, Res, ResMut, Resource},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt, Parent},
    input::{keyboard::KeyCode, Input},
    pbr::{PbrBundle, StandardMaterial},
    prelude::{Deref, DerefMut},
    render::{
        color::Color,
        mesh::{shape, Mesh},
    },
    text::Text,
    transform::components::Transform,
    utils::default,
};

use bevy_mod_picking::{
    events::Out,
    prelude::{Down, ListenerInput, On, Over, Pointer},
};
use ghx_proc_gen::{
    generator::Generator,
    grid::{
        direction::{CoordinateSystem, Direction},
        GridDefinition,
    },
};

use crate::{
    gen::GridNode,
    grid::{
        get_translation_from_grid_coords_3d,
        markers::{GridMarker, MarkerDespawnEvent},
        view::DebugGridView,
    },
};

use super::{
    cursor::{
        cursor_info_to_string, Cursor, CursorBehavior, CursorInfo, CursorMarkerSettings,
        CursorsPanelText, GridCursor, SelectCursor, OVER_CURSOR_SECTION_INDEX,
    },
    generation::{ActiveGeneration, GenerationEvent},
    ProcGenKeyBindings,
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

pub fn insert_cursor_picking_handlers_to_grid_nodes<C: CoordinateSystem>(
    mut commands: Commands,
    spawned_nodes: Query<Entity, Added<GridNode>>,
) {
    use bevy_mod_picking::{pointer::PointerButton, prelude::ListenerMut};

    for entity in spawned_nodes.iter() {
        commands.entity(entity).try_insert((
            On::<Pointer<Over>>::send_event::<NodeOverEvent>(),
            On::<Pointer<Out>>::send_event::<NodeOutEvent>(),
            On::<Pointer<Down>>::run(
                move |event: ListenerMut<Pointer<Down>>,
                      mut selection_events: EventWriter<NodeSelectedEvent>| {
                    if event.button == PointerButton::Primary {
                        selection_events.send(NodeSelectedEvent(event.listener()));
                    }
                },
            ),
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

// TODO Before update_cursors_info_on_cursors_changes and before update_cursors_info_from_generation_events
pub fn update_over_cursor_from_generation_events<C: CoordinateSystem>(
    mut cursors_events: EventReader<GenerationEvent>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut over_cursor: Query<&mut Cursor, With<OverCursor>>,
) {
    let Ok(mut cursor) = over_cursor.get_single_mut() else {
        return;
    };
    for event in cursors_events.read() {
        match event {
            GenerationEvent::Reinitialized(_grid_entity) => {
                // If there is an Over cursor, force despawn it, since we will despawn the underlying node there won't be any NodeOutEvent.
                if let Some(grid_cursor) = &cursor.0 {
                    marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                    cursor.0 = None;
                }
            }
            GenerationEvent::Updated(_grid_entity, _node_index) => {}
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
    mut grid_nodes: Query<(&GridNode, &Parent)>,
    mut cursor: Query<&mut Cursor, With<CB>>,
    generations: Query<(Entity, &GridDefinition<C>), With<Generator<C>>>,
) {
    if let Some(event) = events.read().last() {
        let Ok(mut cursor) = cursor.get_single_mut() else {
            return;
        };
        let Ok((node, node_parent)) = grid_nodes.get_mut(*event.deref()) else {
            return;
        };

        let picked_grid_entity = node_parent.get();
        let update_cursor = match &cursor.0 {
            Some(grid_cursor) => {
                if (grid_cursor.grid != picked_grid_entity) || (grid_cursor.node_index != node.0) {
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

pub fn picking_remove_previous_over_cursor<C: CoordinateSystem>(
    mut out_events: EventReader<NodeOutEvent>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<&GridNode>,
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

#[derive(Resource, Default)]
pub struct CursorTargetAssets {
    target_mesh: Handle<Mesh>,
    target_mat: Handle<StandardMaterial>,
}

pub fn setup_picking_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_target_assets: ResMut<CursorTargetAssets>,
) {
    cursor_target_assets.target_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.9 }));
    cursor_target_assets.target_mat = materials.add(Color::WHITE.with_a(0.5).into());
}

#[derive(Component)]
pub struct CursorTarget;

#[derive(Default)]
pub struct ActiveCursorHelperDirection(pub Option<Direction>);

pub fn update_cursor_targets_nodes<C: CoordinateSystem>(
    mut local_active_cursor_helper_direction: Local<ActiveCursorHelperDirection>,
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    cursor_target_assets: Res<CursorTargetAssets>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    mut over_cursor: Query<&mut Cursor, (With<OverCursor>, Without<SelectCursor>)>,
    grids: Query<(&GridDefinition<C>, &DebugGridView)>,
    cursor_targets: Query<Entity, With<CursorTarget>>,
) {
    let Ok(mut selection_cursor) = selection_cursor.get_single_mut() else {
        return;
    };
    let Some(cursor) = &selection_cursor.0 else {
        return;
    };

    let axis_selection = if keys.pressed(proc_gen_key_bindings.cursor_x_axis) {
        Some(Direction::XForward)
    } else if keys.pressed(proc_gen_key_bindings.cursor_y_axis) {
        Some(Direction::YForward)
    } else if keys.pressed(proc_gen_key_bindings.cursor_z_axis) {
        Some(Direction::ZForward)
    } else {
        None
    };

    if let Some(axis) = axis_selection {
        // Some targeting already active
        if local_active_cursor_helper_direction.0.is_some() {
            return;
        }
        local_active_cursor_helper_direction.0 = Some(axis);

        let Ok((grid, grid_view)) = grids.get(cursor.grid) else {
            return;
        };

        let mut spawn_cursor_target = |x: u32, y: u32, z: u32| {
            let translation = get_translation_from_grid_coords_3d(x, y, z, &grid_view.node_size);
            let helper_node_entity = commands
                .spawn((
                    GridNode(grid.index_from_coords(x, y, z)),
                    CursorTarget,
                    PbrBundle {
                        transform: Transform::from_translation(translation)
                            .with_scale(grid_view.node_size),
                        mesh: cursor_target_assets.target_mesh.clone(),
                        material: cursor_target_assets.target_mat.clone(),
                        ..default()
                    },
                ))
                .id();
            commands.entity(cursor.grid).add_child(helper_node_entity);
        };

        match axis {
            Direction::XForward => {
                for x in 0..grid.size_x() {
                    spawn_cursor_target(x, cursor.position.y, cursor.position.z);
                }
                for y in 0..grid.size_y() {
                    for z in 0..grid.size_z() {
                        spawn_cursor_target(cursor.position.x, y, z);
                    }
                }
            }
            Direction::YForward => {
                for y in 0..grid.size_y() {
                    spawn_cursor_target(cursor.position.x, y, cursor.position.z);
                }
                for x in 0..grid.size_x() {
                    for z in 0..grid.size_z() {
                        spawn_cursor_target(x, cursor.position.y, z);
                    }
                }
            }
            Direction::ZForward => {
                for z in 0..grid.size_z() {
                    spawn_cursor_target(cursor.position.x, cursor.position.y, z);
                }
                for x in 0..grid.size_x() {
                    for y in 0..grid.size_y() {
                        spawn_cursor_target(x, y, cursor.position.z);
                    }
                }
            }
            _ => {}
        }
    } else {
        if local_active_cursor_helper_direction.0.is_some() {
            local_active_cursor_helper_direction.0 = None;
            for cursor_target in cursor_targets.iter() {
                commands.entity(cursor_target).despawn_recursive();
            }
            let Ok(mut over_cursor) = over_cursor.get_single_mut() else {
                return;
            };
            // If there is an Over cursor, force despawn it, since we will despawn the underlying node there won't be any NodeOutEvent.
            if let Some(grid_cursor) = &over_cursor.0 {
                marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                over_cursor.0 = None;
            }
        }
    }
}
