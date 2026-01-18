use bevy::{
    app::App,
    ecs::{
        message::{Message, MessageReader, MessageWriter},
        query::With,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Query, Res, ResMut},
    },
    input::{mouse::MouseButton, ButtonInput},
    prelude::*,
};

#[cfg(feature = "log")]
use bevy::log::warn;

use bevy_egui::{
    egui::{self, Color32, Pos2},
    EguiContexts, EguiPrimaryContextPass,
};
use ghx_proc_gen::{
    generator::{
        model::{ModelInstance, ModelRotation},
        rules::ModelInfo,
        Generator,
    },
    ghx_grid::cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid},
};

use crate::{CursorTarget, GridNode};

use super::{
    cursor::{Cursor, CursorInfo, SelectCursor},
    generation::ActiveGeneration,
    picking::{NodeOverMessage, NodeSelectedMessage},
};

pub(crate) fn plugin<C: CartesianCoordinates>(app: &mut App) {
    app.init_resource::<EditorConfig>()
        .init_resource::<EditorContext>()
        .add_message::<BrushEvent>();

    app.add_systems(
        EguiPrimaryContextPass,
        (
            draw_edition_panel::<C>,
            update_brush,
            update_painting_state,
            paint::<C>,
        )
            .chain()
            .run_if(editor_enabled),
    );
}

/// Resource sued to track the status of the edgui editor
#[derive(Resource)]
pub struct EditorConfig {
    /// Whether or not the editor is currently enabled
    pub enabled: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Context of the egui editor
#[derive(Resource, Default)]
pub struct EditorContext {
    /// Current brush, can be [None]
    pub model_brush: Option<ModelBrush>,
    /// Is the editor currently painting
    pub painting: bool,
}

/// A model "brush" holding information about what model it paints
#[derive(Clone)]
pub struct ModelBrush {
    /// General info about the model
    pub info: ModelInfo,
    /// Specific instance of the model
    pub instance: ModelInstance,
}

/// Event types for model brushes
#[derive(Message)]
pub enum BrushEvent {
    /// Clear the current brush
    ClearBrush,
    /// Update the current brush to a new one
    UpdateBrush(ModelBrush),
    /// Update only the rotation of the current brush
    UpdateRotation(ModelRotation),
}

/// System condition to check if the egui editor is enabled
pub fn editor_enabled(editor_config: Res<EditorConfig>) -> bool {
    editor_config.enabled
}

/// System that can be used to toggle on/off the egui editor
pub fn toggle_editor(mut editor_config: ResMut<EditorConfig>) {
    editor_config.enabled = !editor_config.enabled;
}

/// System used to draw the editor egui window
pub fn draw_edition_panel<C: CartesianCoordinates>(
    editor_context: ResMut<EditorContext>,
    mut contexts: EguiContexts,
    active_generation: Res<ActiveGeneration>,
    mut brush_events: MessageWriter<BrushEvent>,
    generations: Query<&mut Generator<C, CartesianGrid<C>>>,
    selection_cursor: Query<(&Cursor, &CursorInfo), With<SelectCursor>>,
) -> Result {
    let Some(active_generation) = active_generation.0 else {
        return Ok(());
    };
    let Ok(generator) = generations.get(active_generation) else {
        return Ok(());
    };
    let Ok((cursor, cursor_info)) = selection_cursor.single() else {
        return Ok(());
    };

    // TODO Cache ? rules models groups
    egui::Window::new("Edition panel")
        .title_bar(true)
        // TODO Init all those values with viewport size
        .default_pos(Pos2::new(10., 300.))
        .show(contexts.ctx_mut()?, |ui| {
            ui.horizontal_wrapped(|ui| {
                // TODO A rules models display
                ui.label(format!("📖 Rules:",));
                ui.colored_label(
                    Color32::WHITE,
                    format!(
                        "{} models ({} variations)",
                        generator.rules().original_models_count(),
                        generator.rules().models_count(),
                    ),
                );
            });

            match &cursor.0 {
                Some(targeted_node) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("⭕ Selected node: ");
                        ui.colored_label(
                            Color32::WHITE,
                            format!(
                                "{{{}}}, {} possible models ({} variations)",
                                targeted_node.position,
                                cursor_info.models_variations.len(),
                                cursor_info.total_models_count,
                            ),
                        );
                    });
                }
                None => {
                    ui.label("⭕ No selected node");
                }
            };

            ui.separator();
            match &editor_context.model_brush {
                Some(model) => {
                    ui.horizontal(|ui| {
                        ui.label("🖊 Current brush: ");
                        ui.colored_label(
                            Color32::WHITE,
                            format!("{}, {}", model.info.name, model.instance),
                        );
                        if ui.button("Clear").clicked() {
                            brush_events.write(BrushEvent::ClearBrush);
                        }
                    });
                }
                None => {
                    ui.label("🖊 No brush selected");
                }
            };
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for model_group in cursor_info.models_variations.iter() {
                    let selected = match &editor_context.model_brush {
                        Some(model) => model_group.index == model.instance.model_index,
                        None => false,
                    };
                    ui.horizontal(|ui| {
                        let rot_count_tag = if model_group.rotations.len() != 1 {
                            format!(" ({})", model_group.rotations.len())
                        } else {
                            "".to_owned()
                        };
                        if ui
                            .selectable_label(
                                selected,
                                format!("▶ {}{}", model_group.info.name, rot_count_tag,),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(format!(
                                    "{} possible rotations, weight: {}",
                                    model_group.rotations.len(),
                                    model_group.info.weight
                                ));
                            })
                            .clicked()
                        {
                            brush_events.write(BrushEvent::UpdateBrush(ModelBrush {
                                info: model_group.info.clone(),
                                instance: ModelInstance {
                                    model_index: model_group.index,
                                    rotation: model_group.rotations[0],
                                },
                            }));
                        }
                        if selected {
                            for rotation in model_group.rotations.iter() {
                                let is_selected = match &editor_context.model_brush {
                                    Some(model) => *rotation == model.instance.rotation,
                                    None => false,
                                };
                                if ui
                                    .selectable_label(is_selected, format!("{}°", rotation.value()))
                                    .clicked()
                                {
                                    brush_events.write(BrushEvent::UpdateRotation(*rotation));
                                }
                            }
                        }
                    });
                }
            });
        });
    Ok(())
}

/// System reading [BrushEvent] to update the current model brush in the [EditorContext]
pub fn update_brush(
    mut editor_context: ResMut<EditorContext>,
    mut brush_events: MessageReader<BrushEvent>,
) {
    for event in brush_events.read() {
        match event {
            BrushEvent::ClearBrush => editor_context.model_brush = None,
            BrushEvent::UpdateBrush(new_brush) => {
                editor_context.model_brush = Some(new_brush.clone())
            }
            BrushEvent::UpdateRotation(new_rot) => {
                if let Some(brush) = editor_context.model_brush.as_mut() {
                    brush.instance.rotation = *new_rot;
                }
            }
        }
    }
}

/// System updating the painting state in the [EditorContext] based on mouse inputs and [NodeSelectedEvent]
pub fn update_painting_state(
    mut editor_context: ResMut<EditorContext>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut node_select_events: MessageReader<NodeSelectedMessage>,
    cursor_targets: Query<(), With<CursorTarget>>,
) {
    if editor_context.model_brush.is_none() {
        editor_context.painting = false;
        return;
    }
    if let Some(ev) = node_select_events.read().last() {
        if let Ok(_) = cursor_targets.get(ev.0) {
            editor_context.painting = true;
        };
    }
    if buttons.just_released(MouseButton::Left) {
        editor_context.painting = false;
    }
}

/// System issuing the generation requests to the generator based on the painting state
pub fn paint<C: CartesianCoordinates>(
    editor_context: ResMut<EditorContext>,
    active_generation: Res<ActiveGeneration>,
    mut node_over_events: MessageReader<NodeOverMessage>,
    mut generations: Query<&mut Generator<C, CartesianGrid<C>>>,
    cursor_targets: Query<&GridNode, With<CursorTarget>>,
) {
    if !editor_context.painting {
        node_over_events.clear();
        return;
    }
    let Some(model_brush) = &editor_context.model_brush else {
        node_over_events.clear();
        return;
    };
    let Some(active_generation) = active_generation.0 else {
        node_over_events.clear();
        return;
    };
    let Ok(mut generator) = generations.get_mut(active_generation) else {
        node_over_events.clear();
        return;
    };

    for ev in node_over_events.read() {
        let Ok(node) = cursor_targets.get(ev.0) else {
            continue;
        };

        if let Err(err) = generator.set_and_propagate(node.0, model_brush.instance, true) {
            #[cfg(feature = "log")]
            warn!(
                "Failed to generate model {} on node {}: {}",
                model_brush.instance, node.0, err
            );
        }
    }
}
