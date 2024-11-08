use std::{fmt, time::Duration};

use bevy::{
    color::{palettes::css::GREEN, Color},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{EventReader, EventWriter},
        query::{Changed, With, Without},
        system::{Commands, Local, Query, Res, ResMut, Resource},
    },
    hierarchy::BuildChildren,
    input::{keyboard::KeyCode, ButtonInput},
    log::warn,
    prelude::{Text, TextUiWriter},
    render::camera::Camera,
    text::{LineBreak, TextColor, TextFont, TextLayout, TextSpan},
    time::{Time, Timer, TimerMode},
    transform::components::GlobalTransform,
    ui::{BackgroundColor, Node, PositionType, UiRect, Val},
    utils::default,
};
use bevy_ghx_grid::{
    debug_plugin::markers::{spawn_marker, GridMarker, MarkerDespawnEvent},
    ghx_grid::{coordinate_system::CoordinateSystem, direction::Direction},
};
use ghx_proc_gen::{
    generator::{Generator, ModelVariations},
    ghx_grid::cartesian::{
        coordinates::{CartesianCoordinates, CartesianPosition},
        grid::CartesianGrid,
    },
    NodeIndex,
};

#[cfg(feature = "picking")]
use bevy::prelude::PickingBehavior;

use super::{
    generation::{ActiveGeneration, GenerationEvent},
    GridCursorsUiSettings, ProcGenKeyBindings,
};

/// Marker component to be put on a [Camera] to signal that it should be used to display curosr overlays
///
/// - **Not needed** if only a single camera is used.
/// - If used, should not be present on more than 1 camera
#[derive(Component)]
pub struct GridCursorsOverlayCamera;

/// Root marker for the cursors panel UI
#[derive(Component)]
pub struct CursorsPanelRoot;

/// Root marker for the cursors overlay UI
#[derive(Component)]
pub struct CursorsOverlaysRoot;

/// Text component marker for the cursors panel UI
#[derive(Component)]
pub struct CursorsPanelText;

/// Represents a node in a grid and its [GridMarker]
#[derive(Debug)]
pub struct TargetedNode {
    /// Grid entity the node bleongs to
    pub grid: Entity,
    /// Index of the node in its grid
    pub node_index: NodeIndex,
    /// Position of the node in its grid
    pub position: CartesianPosition,
    /// Marker entity for this targeted node
    pub marker: Entity,
}
impl fmt::Display for TargetedNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, index: {}", self.position, self.node_index)
    }
}

/// Represents a generic cursor and its optional target
#[derive(Component, Default, Debug)]
pub struct Cursor(pub Option<TargetedNode>);

/// Information about what is being pointed by a cursor
#[derive(Component, Default, Debug)]
pub struct CursorInfo {
    /// How many possible models for the node pointed by the cursor
    pub total_models_count: u32,
    /// Groups of models for the node pointed by the cursor
    pub models_variations: Vec<ModelVariations>,
}
impl CursorInfo {
    /// Clear all information in the [CursorInfo]
    pub fn clear(&mut self) {
        self.total_models_count = 0;
        self.models_variations.clear();
    }
}

/// Trait implemented by cursors to customize their behavior
pub trait CursorBehavior: Component {
    /// Create a new cursor
    fn new() -> Self;
    /// Returns whether or not this cursor should update the active generation when its target changes
    fn updates_active_gen() -> bool;
}

/// Marker component for a cursor's UI overlay
#[derive(Component, Debug)]
pub struct CursorOverlay {
    /// The cursor Entity
    pub cursor_entity: Entity,
}

/// Trait implemented by cursors settings resources
pub trait CursorMarkerSettings: Resource {
    /// Returns the color used for this type of cursor
    fn color(&self) -> Color;
}

/// Settings for the selection cursor
#[derive(Resource)]
pub struct SelectionCursorMarkerSettings(pub Color);
impl Default for SelectionCursorMarkerSettings {
    fn default() -> Self {
        Self(Color::Srgba(GREEN))
    }
}
impl CursorMarkerSettings for SelectionCursorMarkerSettings {
    fn color(&self) -> Color {
        self.0
    }
}

/// Selection cursor marker component
#[derive(Component, Debug)]
pub struct SelectCursor;
impl CursorBehavior for SelectCursor {
    fn new() -> Self {
        Self
    }
    fn updates_active_gen() -> bool {
        true
    }
}

/// Used to index text sections when displaying cursors Ui in a panel
pub const OVER_CURSOR_SECTION_INDEX: usize = 0;
/// Used to index text sections when displaying cursors Ui in a panel
pub const SELECTION_CURSOR_SECTION_INDEX: usize = 1;

/// Setup system used to spawn the cursors UI panel
pub fn setup_cursors_panel(mut commands: Commands, ui_config: Res<GridCursorsUiSettings>) {
    let root = commands
        .spawn((
            CursorsPanelRoot,
            Name::new("CursorsPanelRoot"),
            BackgroundColor(ui_config.background_color),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Percent(1.),
                bottom: Val::Percent(1.),
                top: Val::Auto,
                left: Val::Auto,
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn((
            CursorsPanelText,
            TextLayout::default(),
            TextFont {
                font_size: ui_config.font_size,
                ..Default::default()
            },
            TextColor(ui_config.text_color),
        ))
        // TODO TextSpan Does TextSpan needs text font and color ?
        // Over cursor
        .with_child((TextSpan::new(" N/A"),))
        // Selection cursor
        .with_child((TextSpan::new(" N/A"),))
        .id();

    commands.entity(root).add_child(text);
}

/// Setpu system used to spawn the cursors UI overlay root
pub fn setup_cursors_overlays(mut commands: Commands) {
    let root = commands
        .spawn((
            CursorsOverlaysRoot,
            Name::new("CursorsOverlaysRoot"),
            Node { ..default() },
        ))
        .id();
    #[cfg(feature = "picking")]
    commands.entity(root).insert(PickingBehavior::IGNORE);
}

/// Setup system to spawn a cursor and its overlay
pub fn setup_cursor<C: CoordinateSystem, CI: CursorBehavior>(
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

    let cursor_overlay_entity = commands.spawn(CursorOverlay { cursor_entity }).id();
    commands.entity(root).add_child(cursor_overlay_entity);

    #[cfg(feature = "picking")]
    commands
        .entity(cursor_overlay_entity)
        .insert(PickingBehavior::IGNORE);
}

/// System updating all the [CursorInfo] components when [Cursor] components are changed
pub fn update_cursors_info_on_cursors_changes<C: CartesianCoordinates>(
    mut moved_cursors: Query<(&mut CursorInfo, &Cursor), Changed<Cursor>>,
    generators: Query<&Generator<C, CartesianGrid<C>>>,
) {
    for (mut cursor_info, cursor) in moved_cursors.iter_mut() {
        match &cursor.0 {
            Some(grid_cursor) => {
                if let Ok(generator) = generators.get(grid_cursor.grid) {
                    (
                        cursor_info.models_variations,
                        cursor_info.total_models_count,
                    ) = generator.get_models_variations_on(grid_cursor.node_index);
                }
            }
            None => cursor_info.clear(),
        }
    }
}

/// System updating all the [CursorInfo] based on [GenerationEvent]
pub fn update_cursors_info_from_generation_events<C: CartesianCoordinates>(
    mut cursors_events: EventReader<GenerationEvent>,
    generators: Query<&Generator<C, CartesianGrid<C>>>,
    mut cursors: Query<(&Cursor, &mut CursorInfo)>,
) {
    for event in cursors_events.read() {
        for (cursor, mut cursor_info) in cursors.iter_mut() {
            let Some(grid_cursor) = &cursor.0 else {
                continue;
            };

            match event {
                GenerationEvent::Reinitialized(grid_entity) => {
                    let Ok(generator) = generators.get(*grid_entity) else {
                        continue;
                    };
                    (
                        cursor_info.models_variations,
                        cursor_info.total_models_count,
                    ) = generator.get_models_variations_on(grid_cursor.node_index);
                }
                GenerationEvent::Updated(grid_entity, node_index) => {
                    let Ok(generator) = generators.get(*grid_entity) else {
                        continue;
                    };
                    if grid_cursor.node_index == *node_index {
                        (
                            cursor_info.models_variations,
                            cursor_info.total_models_count,
                        ) = generator.get_models_variations_on(grid_cursor.node_index);
                    }
                }
            }
        }
    }
}

/// System updating the selection cursor panel UI based on changes in [CursorInfo]
pub fn update_selection_cursor_panel_text(
    mut writer: TextUiWriter,
    mut cursors_panel_text: Query<Entity, With<CursorsPanelText>>,
    updated_cursors: Query<(&CursorInfo, &Cursor), (Changed<CursorInfo>, With<SelectCursor>)>,
) {
    if let Ok((cursor_info, cursor)) = updated_cursors.get_single() {
        for panel_entity in &mut cursors_panel_text {
            let mut ui_text = writer.text(panel_entity, SELECTION_CURSOR_SECTION_INDEX);
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

/// Listen to [KeyCode] to deselect the current selection cursor
pub fn deselect_from_keybinds(
    keys: Res<ButtonInput<KeyCode>>,
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

/// Simple entity collection
pub struct EntityProvider {
    /// Entities in the collection
    pub entities: Vec<Entity>,
    /// Current index in the collection
    pub index: usize,
}

impl Default for EntityProvider {
    fn default() -> Self {
        Self {
            entities: Vec::new(),
            index: 0,
        }
    }
}
impl EntityProvider {
    /// Updates the collection with `entities` and clamp the index to the new collection length
    pub fn update(&mut self, entities: Vec<Entity>) {
        self.entities = entities;
        self.index = (self.index + 1) % self.entities.len();
    }

    /// Returns the entity at the current index
    pub fn get(&self) -> Entity {
        self.entities[self.index]
    }
}

/// System that listens to the generation switch [KeyCode] to switch the current active generation grid
pub fn switch_generation_selection_from_keybinds<C: CartesianCoordinates>(
    mut local_grid_cycler: Local<EntityProvider>,
    mut commands: Commands,
    mut active_generation: ResMut<ActiveGeneration>,
    keys: Res<ButtonInput<KeyCode>>,
    selection_marker_settings: Res<SelectionCursorMarkerSettings>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    generators: Query<Entity, (With<Generator<C, CartesianGrid<C>>>, With<CartesianGrid<C>>)>,
) {
    if keys.just_pressed(proc_gen_key_bindings.switch_grid) {
        let Ok(mut cursor) = selection_cursor.get_single_mut() else {
            return;
        };

        local_grid_cycler.update(generators.iter().collect());
        let grid_entity = local_grid_cycler.get();
        active_generation.0 = Some(grid_entity);
        // Despawn previous if any
        if let Some(grid_cursor) = &cursor.0 {
            marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
        }
        // Spawn on new selected grid
        cursor.0 = Some(spawn_marker_and_create_cursor(
            &mut commands,
            grid_entity,
            CartesianPosition::new(0, 0, 0),
            0,
            selection_marker_settings.color(),
        ));
    }
}

const CURSOR_KEYS_MOVEMENT_COOLDOWN_MS: u64 = 140;
const CURSOR_KEYS_MOVEMENT_SHORT_COOLDOWN_MS: u64 = 45;
const CURSOR_KEYS_MOVEMENT_SPEED_UP_DELAY_MS: u64 = 350;

/// Resource used to customize keyboard movement of the selection cursor
#[derive(Resource)]
pub struct CursorKeyboardMovementSettings {
    /// Cooldown between two movements when not sped up
    pub default_cooldown_ms: u64,
    /// Cooldown between two movements when sped up
    pub short_cooldown_ms: u64,
    /// Duration after which the cooldown between two movmeents gets sped up if
    /// the move key is continuously pressed
    pub speed_up_timer_duration_ms: Duration,
}

impl Default for CursorKeyboardMovementSettings {
    fn default() -> Self {
        Self {
            default_cooldown_ms: CURSOR_KEYS_MOVEMENT_COOLDOWN_MS,
            short_cooldown_ms: CURSOR_KEYS_MOVEMENT_SHORT_COOLDOWN_MS,
            speed_up_timer_duration_ms: Duration::from_millis(
                CURSOR_KEYS_MOVEMENT_SPEED_UP_DELAY_MS,
            ),
        }
    }
}

/// Resource used to track keyboard movement variables for the selection cursor
#[derive(Resource)]
pub struct CursorKeyboardMovement {
    /// Current cooldwon to move again
    pub cooldown: Timer,
    /// Current timer before speeding up the movements
    pub speed_up_timer: Timer,
}

impl Default for CursorKeyboardMovement {
    fn default() -> Self {
        Self {
            cooldown: Timer::new(
                Duration::from_millis(CURSOR_KEYS_MOVEMENT_COOLDOWN_MS),
                TimerMode::Once,
            ),
            speed_up_timer: Timer::new(
                Duration::from_millis(CURSOR_KEYS_MOVEMENT_SPEED_UP_DELAY_MS),
                TimerMode::Once,
            ),
        }
    }
}

/// System handling movements of the selection cursor from the keyboard
pub fn move_selection_from_keybinds<C: CartesianCoordinates>(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    selection_marker_settings: Res<SelectionCursorMarkerSettings>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    key_mvmt_values: Res<CursorKeyboardMovementSettings>,
    mut key_mvmt: ResMut<CursorKeyboardMovement>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    grids: Query<(Entity, &CartesianGrid<C>)>,
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
        // Just pressed => moves
        // Pressed => moves with default cooldown
        // Pressed for a while => speeds up, shorter cooldown
        // Sped up & no press => resets to default cooldown
        let cursor_movement = if keys.just_pressed(proc_gen_key_bindings.prev_node) {
            Some(-1)
        } else if keys.just_pressed(proc_gen_key_bindings.next_node) {
            Some(1)
        } else {
            let (movement, pressed) = match key_mvmt.cooldown.finished() {
                true => {
                    if keys.pressed(proc_gen_key_bindings.prev_node) {
                        (Some(-1), true)
                    } else if keys.pressed(proc_gen_key_bindings.next_node) {
                        (Some(1), true)
                    } else {
                        (None, false)
                    }
                }
                false => {
                    if keys.pressed(proc_gen_key_bindings.prev_node)
                        || keys.pressed(proc_gen_key_bindings.next_node)
                    {
                        (None, true)
                    } else {
                        (None, false)
                    }
                }
            };
            if pressed {
                key_mvmt.cooldown.tick(time.delta());
                if !key_mvmt.speed_up_timer.finished() {
                    key_mvmt.speed_up_timer.tick(time.delta());
                } else if key_mvmt.speed_up_timer.just_finished() {
                    key_mvmt
                        .cooldown
                        .set_duration(Duration::from_millis(key_mvmt_values.short_cooldown_ms));
                }
            } else {
                if key_mvmt.speed_up_timer.finished() {
                    key_mvmt
                        .cooldown
                        .set_duration(Duration::from_millis(key_mvmt_values.default_cooldown_ms));
                }
                key_mvmt
                    .speed_up_timer
                    .set_duration(key_mvmt_values.speed_up_timer_duration_ms);
                key_mvmt.speed_up_timer.reset();
            }
            movement
        };

        if let Some(movement) = cursor_movement {
            key_mvmt.cooldown.reset();

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
                    Some((grid_entity, 0, CartesianPosition::new(0, 0, 0)))
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

/// Utility function to spanw a [GridMarker]
pub fn spawn_marker_and_create_cursor(
    commands: &mut Commands,
    grid_entity: Entity,
    position: CartesianPosition,
    node_index: NodeIndex,
    color: Color,
) -> TargetedNode {
    let marker = spawn_marker(commands, grid_entity, color, position);
    TargetedNode {
        grid: grid_entity,
        node_index,
        position,
        marker,
    }
}

/// Utility function to transform data from a [CursorInfo] into a [String]
pub fn cursor_info_to_string(cursor: &TargetedNode, cursor_info: &CursorInfo) -> String {
    let text = if cursor_info.models_variations.len() > 1 {
        format!(
            "Grid: {{{}}}\n\
            {} possible models, {} variations:\n\
            {{{}}}\n\
            {{{}}}\n\
            {}",
            cursor,
            cursor_info.models_variations.len(),
            cursor_info.total_models_count,
            cursor_info.models_variations[0],
            cursor_info.models_variations[1],
            if cursor_info.models_variations.len() > 2 {
                "...\n"
            } else {
                ""
            }
        )
    } else if cursor_info.models_variations.len() == 1 {
        format!(
            "Grid: {{{}}}\n\
            Model: {{{}}}\n",
            cursor, cursor_info.models_variations[0],
        )
    } else {
        format!(
            "Grid: {{{}}}\n\
            No models possible\n",
            cursor,
        )
    };
    text
}

/// Local flag used as a system local resource
#[derive(Default)]
pub struct Flag(pub bool);

/// System updating the cursors overlay UI
pub fn update_cursors_overlays(
    mut camera_warning_flag: Local<Flag>,
    mut commands: Commands,
    ui_config: Res<GridCursorsUiSettings>,
    just_one_camera: Query<(&Camera, &GlobalTransform), Without<GridCursorsOverlayCamera>>,
    overlay_camera: Query<(&Camera, &GlobalTransform), With<GridCursorsOverlayCamera>>,
    cursor_overlays: Query<(Entity, &CursorOverlay)>,
    cursors: Query<(&CursorInfo, &Cursor)>,
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
        // TODO Bevy 0.15 might need to tweak the behavior to remove/update some components when there is no overlay
        let Some(grid_cursor) = &cursor.0 else {
            // No cursor => no text overlay
            commands.entity(overlay_entity).insert(Text::default());
            continue;
        };
        let Ok(marker_gtransform) = markers.get(grid_cursor.marker) else {
            // No marker => no text overlay
            commands.entity(overlay_entity).insert(Text::default());
            continue;
        };
        let Ok(viewport_pos) =
            camera.world_to_viewport(cam_gtransform, marker_gtransform.translation())
        else {
            continue;
        };

        let text = cursor_info_to_string(&grid_cursor, cursor_info);
        commands.entity(overlay_entity).insert((
            // TODO bevy 0.15: Insert background once ?
            BackgroundColor(ui_config.background_color),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextColor(ui_config.text_color),
            TextFont {
                font_size: ui_config.font_size,
                ..default()
            },
            Text(text),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(viewport_pos.x + 5.0),
                top: Val::Px(viewport_pos.y + 5.0),
                ..default()
            },
        ));
    }
}
