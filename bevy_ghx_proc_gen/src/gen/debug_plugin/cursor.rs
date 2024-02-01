use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::EventWriter,
        query::{Changed, With, Without},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::BuildChildren,
    input::{keyboard::KeyCode, Input},
    prelude::{Deref, DerefMut},
    render::color::Color,
    text::{Text, TextSection, TextStyle},
    time::{Time, Timer},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        BackgroundColor, PositionType, Style, UiRect, Val,
    },
    utils::default,
};
use ghx_proc_gen::{
    generator::{rules::ModelInfo, Generator},
    grid::{
        direction::{CoordinateSystem, Direction},
        GridDefinition, GridPosition, NodeIndex,
    },
};

use crate::grid::markers::{GridMarker, MarkerDespawnEvent};

use super::ProcGenKeyBindings;

#[derive(Component)]
pub struct SelectionCursorUiRoot;

#[derive(Component)]
pub struct SelectionCursorText;

pub fn setup_selection_cursor_info_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            SelectionCursorUiRoot,
            NodeBundle {
                background_color: BackgroundColor(Color::BLACK.with_a(0.5)),
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
            SelectionCursorText,
            TextBundle {
                text: Text::from_sections([TextSection {
                    value: " N/A".into(),
                    style: TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                }]),
                ..Default::default()
            },
        ))
        .id();
    commands.entity(root).add_child(text);
}

pub fn insert_selection_cursor_to_new_generations<C: CoordinateSystem>(
    mut commands: Commands,
    mut new_generations: Query<
        (Entity, &GridDefinition<C>, &Generator<C>),
        Without<GridSelectionCursor>,
    >,
) {
    for (gen_entity, _grid, _generation) in new_generations.iter_mut() {
        commands.entity(gen_entity).insert((
            ActiveGridCursor,
            GridSelectionCursor(GridCursor {
                color: Color::GREEN,
                node_index: 0,
                position: GridPosition::new(0, 0, 0),
                marker: None,
            }),
            GridSelectionCursorInfo(GridCursorInfo::new()),
        ));
    }
}

#[derive(Component)]
pub struct ActiveGridCursor;

#[derive(Debug)]
pub struct GridCursor {
    pub color: Color,
    pub node_index: NodeIndex,
    pub position: GridPosition,
    pub marker: Option<Entity>,
}
impl fmt::Display for GridCursor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, index: {}", self.position, self.node_index)
    }
}

#[derive(Component, Debug, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct GridSelectionCursor(pub GridCursor);

#[derive(Debug)]
pub struct GridCursorInfo {
    models: Vec<ModelInfo>,
}
impl GridCursorInfo {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }
}

#[derive(Component, Debug, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct GridSelectionCursorInfo(pub GridCursorInfo);

pub fn update_grid_cursor_info_on_changes<
    C: CoordinateSystem,
    GC: Component + Deref<Target = GridCursor>,
    GCI: Component + DerefMut<Target = GridCursorInfo>,
>(
    mut moved_cursors: Query<(&Generator<C>, &mut GCI, &GC), Changed<GC>>,
) {
    for (generator, mut cursor_info, cursor) in moved_cursors.iter_mut() {
        cursor_info.models = generator.get_models_info_on(cursor.node_index);
    }
}

pub fn update_selection_cursor_info_ui(
    mut selection_cursor_text: Query<&mut Text, With<SelectionCursorText>>,
    mut moved_selection_cursors: Query<
        (
            &GridSelectionCursorInfo,
            &GridSelectionCursor,
            &ActiveGridCursor,
        ),
        Changed<GridSelectionCursorInfo>,
    >,
) {
    if let Ok((cursor_info, cursor, _active)) = moved_selection_cursors.get_single() {
        for mut text in &mut selection_cursor_text {
            if cursor_info.models.len() > 1 {
                text.sections[0].value = format!(
                    "Grid: {{{}}}\n\
                    {} possible models:\n\
                    {{{}}}\n\
                    {{{}}}\n\
                    ...",
                    cursor.0,
                    cursor_info.models.len(),
                    cursor_info.models[0],
                    cursor_info.models[1],
                );
            } else if cursor_info.models.len() == 1 {
                text.sections[0].value = format!(
                    "Grid: {{{}}}\n\
                    Model: {{{}}}",
                    cursor.0, cursor_info.models[0],
                );
            } else {
                text.sections[0].value = format!(
                    "Grid: {{{}}}\n\
                    No models",
                    cursor.0,
                );
            }
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct CursorMoveCooldown(pub Timer);

pub fn keybinds_update_grid_selection_cursor_position<C: CoordinateSystem>(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut move_cooldown: ResMut<CursorMoveCooldown>,
    mut active_grid_cursors: Query<
        (Entity, &GridDefinition<C>, &mut GridSelectionCursor),
        With<ActiveGridCursor>,
    >,
) {
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

            for (grid_entity, grid, mut cursor) in active_grid_cursors.iter_mut() {
                match grid.get_index_in_direction(&cursor.position, axis, movement) {
                    Some(node_index) => {
                        if let Some(previous_cursor_entity) = cursor.marker {
                            marker_events.send(MarkerDespawnEvent::Remove {
                                marker_entity: previous_cursor_entity,
                            });
                        }
                        cursor.node_index = node_index;
                        cursor.position = grid.pos_from_index(node_index);
                        let marker_entity = commands
                            .spawn(GridMarker::new(cursor.color, cursor.position.clone()))
                            .id();
                        commands.entity(grid_entity).add_child(marker_entity);
                        cursor.marker = Some(marker_entity);
                    }
                    None => (),
                }
            }
        }
    }
}
