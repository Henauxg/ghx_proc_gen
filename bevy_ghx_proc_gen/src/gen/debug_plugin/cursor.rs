use std::fmt;

use bevy::{
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{EventReader, EventWriter},
        query::{Changed, With, Without},
        system::{Commands, Local, Query, Res, ResMut, Resource},
    },
    hierarchy::BuildChildren,
    input::{keyboard::KeyCode, Input},
    log::warn,
    prelude::{Deref, DerefMut},
    render::{camera::Camera, color::Color},
    text::{BreakLineOn, Text, TextSection, TextStyle},
    time::{Time, Timer},
    transform::components::GlobalTransform,
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        BackgroundColor, PositionType, Style, UiRect, Val,
    },
    utils::default,
};
use bevy_mod_picking::picking_core::Pickable;
use ghx_proc_gen::{
    generator::{rules::ModelInfo, Generator},
    grid::{
        direction::{CoordinateSystem, Direction},
        GridDefinition, GridPosition, NodeIndex,
    },
};

use crate::grid::markers::{spawn_marker, GridMarker, MarkerDespawnEvent};

use super::{generation::GenerationEvent, GridCursorsUiSettings, ProcGenKeyBindings};

#[derive(Component)]
pub struct GridCursorsOverlayCamera;

#[derive(Component)]
pub struct CursorsPanelRoot;

#[derive(Component)]
pub struct CursorsOverlaysRoot;

#[derive(Component)]
pub struct CursorsPanelText;

#[derive(Debug)]
pub struct GridCursor {
    pub grid: Entity,
    pub node_index: NodeIndex,
    pub position: GridPosition,
    pub marker: Entity,
}
impl fmt::Display for GridCursor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, index: {}", self.position, self.node_index)
    }
}

#[derive(Component, Default, Debug)]
pub struct Cursor(pub Option<GridCursor>);

#[derive(Component, Default, Debug)]
pub struct CursorInfo {
    pub models: Vec<ModelInfo>,
}

pub trait CursorIdentifier: Component {
    fn new() -> Self;
}

#[derive(Component, Debug)]
pub struct CursorOverlay {
    pub cursor_entity: Entity,
}

pub trait CursorMarkerSettings: Resource {
    fn color(&self) -> Color;
}

#[derive(Resource)]
pub struct SelectionCursorMarkerSettings(pub Color);
impl Default for SelectionCursorMarkerSettings {
    fn default() -> Self {
        Self(Color::GREEN)
    }
}
impl CursorMarkerSettings for SelectionCursorMarkerSettings {
    fn color(&self) -> Color {
        self.0
    }
}

#[derive(Component, Debug)]
pub struct SelectCursor;
impl CursorIdentifier for SelectCursor {
    fn new() -> Self {
        Self
    }
}

pub const OVER_CURSOR_SECTION_INDEX: usize = 0;
pub const SELECTION_CURSOR_SECTION_INDEX: usize = 1;

pub fn setup_cursors_panel(mut commands: Commands, ui_config: Res<GridCursorsUiSettings>) {
    let root = commands
        .spawn((
            CursorsPanelRoot,
            Name::new("CursorsPanelRoot"),
            NodeBundle {
                background_color: BackgroundColor(ui_config.background_color),
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(1.),
                    bottom: Val::Percent(1.),
                    top: Val::Auto,
                    left: Val::Auto,
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn((
            CursorsPanelText,
            TextBundle {
                text: Text::from_sections([
                    // Over cursor
                    TextSection {
                        value: " N/A".into(),
                        style: TextStyle {
                            font_size: ui_config.font_size,
                            color: ui_config.text_color,
                            ..default()
                        },
                    },
                    // Selection cursor
                    TextSection {
                        value: " N/A".into(),
                        style: TextStyle {
                            font_size: ui_config.font_size,
                            color: ui_config.text_color,
                            ..default()
                        },
                    },
                ]),
                ..Default::default()
            },
        ))
        .id();
    commands.entity(root).add_child(text);
}

pub fn setup_cursors_overlays(mut commands: Commands) {
    let root = commands
        .spawn((
            CursorsOverlaysRoot,
            Name::new("CursorsOverlaysRoot"),
            NodeBundle { ..default() },
        ))
        .id();
    #[cfg(feature = "picking")]
    commands.entity(root).insert(Pickable::IGNORE);
}

pub fn setup_cursor<C: CoordinateSystem, CI: CursorIdentifier>(
    mut commands: Commands,
    overlays_root: Query<Entity, With<CursorsOverlaysRoot>>,
) {
    let cursor_entity = commands
        .spawn((Cursor::default(), CursorInfo::default(), CI::new()))
        .id();

    let Ok(root) = overlays_root.get_single() else {
        // No overlays
        return;
    };

    let cursor_overlay_entity = commands
        .spawn((
            CursorOverlay { cursor_entity },
            // https://github.com/bevyengine/bevy/issues/11572
            // If we only add the node later, Bevy panics in 0.12.1
            TextBundle { ..default() },
        ))
        .id();
    commands.entity(root).add_child(cursor_overlay_entity);

    #[cfg(feature = "picking")]
    commands
        .entity(cursor_overlay_entity)
        .insert(Pickable::IGNORE);
}

pub fn update_cursors_info_on_cursors_changes<C: CoordinateSystem>(
    mut moved_cursors: Query<(&mut CursorInfo, &Cursor), Changed<Cursor>>,
    generators: Query<&Generator<C>>,
) {
    for (mut cursor_info, cursor) in moved_cursors.iter_mut() {
        match &cursor.0 {
            Some(grid_cursor) => {
                if let Ok(generator) = generators.get(grid_cursor.grid) {
                    cursor_info.models = generator.get_models_info_on(grid_cursor.node_index)
                }
            }
            None => cursor_info.models.clear(),
        }
    }
}

pub fn update_cursor_from_generation_events<C: CoordinateSystem>(
    mut cursors_events: EventReader<GenerationEvent>,
    generators: Query<&Generator<C>>,
    mut cursors: Query<(&Cursor, &mut CursorInfo)>,
) {
    for (cursor, mut cursor_info) in cursors.iter_mut() {
        let Some(grid_cursor) = &cursor.0 else {
            return;
        };
        for event in cursors_events.read() {
            match event {
                GenerationEvent::Reinitialized(grid_entity) => {
                    let Ok(generator) = generators.get(*grid_entity) else {
                        continue;
                    };
                    cursor_info.models = generator.get_models_info_on(grid_cursor.node_index);
                }
                GenerationEvent::Updated(grid_entity, node_index) => {
                    let Ok(generator) = generators.get(*grid_entity) else {
                        continue;
                    };
                    if grid_cursor.node_index == *node_index {
                        cursor_info.models = generator.get_models_info_on(grid_cursor.node_index);
                    }
                }
            }
        }
    }
}

pub fn update_selection_cursor_panel_text(
    mut cursors_panel_text: Query<&mut Text, With<CursorsPanelText>>,
    mut updated_cursors: Query<(&CursorInfo, &Cursor), (Changed<CursorInfo>, With<SelectCursor>)>,
) {
    if let Ok((cursor_info, cursor)) = updated_cursors.get_single() {
        for mut text in &mut cursors_panel_text {
            let ui_text = &mut text.sections[SELECTION_CURSOR_SECTION_INDEX].value;
            match &cursor.0 {
                Some(grid_cursor) => {
                    *ui_text = format!(
                        "Selected:\n{}",
                        cursor_info_to_string(grid_cursor, cursor_info)
                    );
                }
                None => ui_text.clear(),
            }
        }
    }
}

pub fn deselect_from_keybinds(
    keys: Res<Input<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
) {
    if keys.just_pressed(proc_gen_key_bindings.deselect) {
        let Ok(mut cursor) = selection_cursor.get_single_mut() else {
            return;
        };

        if let Some(grid_cursor) = &cursor.0 {
            marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
            cursor.0 = None;
        }
    }
}

#[derive(Default)]
pub struct GridIndexStorage(usize);

pub fn switch_grid_selection_from_keybinds<C: CoordinateSystem>(
    mut local_grid_index: Local<GridIndexStorage>,
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    selection_marker_settings: Res<SelectionCursorMarkerSettings>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    grids: Query<Entity, With<GridDefinition<C>>>,
) {
    if keys.just_pressed(proc_gen_key_bindings.switch_grid) {
        let Ok(mut cursor) = selection_cursor.get_single_mut() else {
            return;
        };

        let all_grids: Vec<Entity> = grids.iter().collect();
        local_grid_index.0 = (local_grid_index.0 + 1) % all_grids.len();
        let grid_entity = all_grids[local_grid_index.0];
        // Despawn previous if any
        if let Some(grid_cursor) = &cursor.0 {
            marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
        }
        // Spawn on new selected grid
        cursor.0 = Some(spawn_marker_and_create_cursor(
            &mut commands,
            grid_entity,
            GridPosition::new(0, 0, 0),
            0,
            selection_marker_settings.color(),
        ));
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct CursorKeyboardMoveCooldown(pub Timer);

pub fn move_selection_from_keybinds<C: CoordinateSystem>(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    selection_marker_settings: Res<SelectionCursorMarkerSettings>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut move_cooldown: ResMut<CursorKeyboardMoveCooldown>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    grids: Query<(Entity, &GridDefinition<C>)>,
) {
    let Ok(mut cursor) = selection_cursor.get_single_mut() else {
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
        move_cooldown.tick(time.delta());
        let cursor_movement = match move_cooldown.finished() {
            true => {
                if keys.pressed(proc_gen_key_bindings.prev_node) {
                    Some(-1)
                } else if keys.pressed(proc_gen_key_bindings.next_node) {
                    Some(1)
                } else {
                    None
                }
            }
            false => None,
        };

        if let Some(movement) = cursor_movement {
            move_cooldown.reset();

            let update_cursor = match &cursor.0 {
                Some(grid_cursor) => {
                    let Ok((_grid_entity, grid)) = grids.get(grid_cursor.grid) else {
                        return;
                    };
                    match grid.get_index_in_direction(&grid_cursor.position, axis, movement) {
                        Some(node_index) => {
                            marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
                            Some((
                                grid_cursor.grid,
                                node_index,
                                grid.pos_from_index(node_index),
                            ))
                        }
                        None => None,
                    }
                }
                None => {
                    // Currently no selection cursor, spawn it on the last Grid
                    let Some((grid_entity, _grid)) = grids.iter().last() else {
                        return;
                    };
                    Some((grid_entity, 0, GridPosition::new(0, 0, 0)))
                }
            };

            match update_cursor {
                Some((grid_entity, node_index, position)) => {
                    cursor.0 = Some(spawn_marker_and_create_cursor(
                        &mut commands,
                        grid_entity,
                        position,
                        node_index,
                        selection_marker_settings.color(),
                    ));
                }
                None => (),
            }
        }
    }
}

pub fn spawn_marker_and_create_cursor(
    commands: &mut Commands,
    grid_entity: Entity,
    position: GridPosition,
    node_index: NodeIndex,
    color: Color,
) -> GridCursor {
    let marker = spawn_marker(commands, grid_entity, color, position);
    GridCursor {
        grid: grid_entity,
        node_index,
        position,
        marker,
    }
}

pub fn cursor_info_to_string(cursor: &GridCursor, cursor_info: &CursorInfo) -> String {
    let text = if cursor_info.models.len() > 1 {
        format!(
            "Grid: {{{}}}\n\
            {} possible models:\n\
            {{{}}}\n\
            {{{}}}\n\
            ...\n",
            cursor,
            cursor_info.models.len(),
            cursor_info.models[0],
            cursor_info.models[1],
        )
    } else if cursor_info.models.len() == 1 {
        format!(
            "Grid: {{{}}}\n\
            Model: {{{}}}\n",
            cursor, cursor_info.models[0],
        )
    } else {
        format!(
            "Grid: {{{}}}\n\
            No models\n",
            cursor,
        )
    };
    text
}

#[derive(Default)]
pub struct Flag(pub bool);

pub fn update_cursors_overlays(
    mut camera_warning_flag: Local<Flag>,
    mut commands: Commands,
    ui_config: Res<GridCursorsUiSettings>,
    just_one_camera: Query<(&Camera, &GlobalTransform), Without<GridCursorsOverlayCamera>>,
    overlay_camera: Query<(&Camera, &GlobalTransform), With<GridCursorsOverlayCamera>>,
    mut cursor_overlays: Query<(Entity, &CursorOverlay)>,
    mut cursors: Query<(&CursorInfo, &Cursor)>,
    markers: Query<&GlobalTransform, With<GridMarker>>,
) {
    let (camera, cam_gtransform) = match just_one_camera.get_single() {
        Ok(found) => found,
        Err(_) => match overlay_camera.get_single() {
            Ok(found) => found,
            Err(_) => {
                if !camera_warning_flag.0 {
                    warn!("None (or too many) Camera(s) found with 'GridCursorsOverlayCamera' component to display cursors overlays. Add `GridCursorsOverlayCamera` component to a Camera or change the cursor UI mode.");
                    camera_warning_flag.0 = true;
                }
                return;
            }
        },
    };

    for (overlay_entity, overlay) in cursor_overlays.iter() {
        let Ok((cursor_info, cursor)) = cursors.get(overlay.cursor_entity) else {
            continue;
        };
        let Some(grid_cursor) = &cursor.0 else {
            // No cursor => no text overlay
            commands.entity(overlay_entity).insert(TextBundle {
                ..Default::default()
            });
            continue;
        };
        let Ok(marker_gtransform) = markers.get(grid_cursor.marker) else {
            // No marker => no text overlay
            commands.entity(overlay_entity).insert(TextBundle {
                ..Default::default()
            });
            continue;
        };
        let Some(viewport_pos) =
            camera.world_to_viewport(cam_gtransform, marker_gtransform.translation())
        else {
            continue;
        };

        let text = cursor_info_to_string(&grid_cursor, cursor_info);
        commands.entity(overlay_entity).insert(TextBundle {
            background_color: BackgroundColor(ui_config.background_color),
            text: Text {
                linebreak_behavior: BreakLineOn::NoWrap,
                sections: vec![TextSection {
                    value: text,
                    style: TextStyle {
                        font_size: ui_config.font_size,
                        color: ui_config.text_color,
                        ..Default::default()
                    },
                }],
                ..Default::default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(viewport_pos.x - 5.0),
                top: Val::Px(viewport_pos.y - 5.0),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}
