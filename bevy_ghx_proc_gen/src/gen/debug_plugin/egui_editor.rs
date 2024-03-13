use bevy::{
    ecs::{
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::With,
        system::{Query, Res, ResMut, Resource},
    },
    input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput},
    log::warn,
};
use bevy_egui::{
    egui::{self, Color32, Pos2},
    EguiContexts,
};
use bevy_ghx_grid::ghx_grid::coordinate_system::CoordinateSystem;
use ghx_proc_gen::generator::{
    model::{ModelIndex, ModelInstance, ModelRotation},
    rules::ModelInfo,
    Generator,
};

use crate::gen::GridNode;

use super::{
    cursor::{Cursor, CursorInfo, SelectCursor, TargetedNode},
    generation::ActiveGeneration,
    picking::{CursorTarget, NodeOverEvent, NodeSelectedEvent},
};

#[derive(Resource, Default)]
pub struct EditorContext {
    pub model_brush: Option<ModelBrush>,
    pub painting: bool,
}

#[derive(Clone)]
pub struct ModelBrush {
    info: ModelInfo,
    instance: ModelInstance,
}

#[derive(Event)]
pub enum BrushEvent {
    ClearBrush,
    UpdateBrush(ModelBrush),
    UpdateRotation(ModelRotation),
}

pub fn draw_edition_panel<C: CoordinateSystem>(
    mut editor_context: ResMut<EditorContext>,
    mut contexts: EguiContexts,
    active_generation: Res<ActiveGeneration>,
    mut brush_events: EventWriter<BrushEvent>,
    mut generations: Query<&mut Generator<C>>,
    selection_cursor: Query<(&Cursor, &CursorInfo), With<SelectCursor>>,
) {
    let Some(active_generation) = active_generation.0 else {
        return;
    };
    let Ok(generator) = generations.get(active_generation) else {
        return;
    };
    let Ok((cursor, cursor_info)) = selection_cursor.get_single() else {
        return;
    };

    // TODO Cache ? rules models groups
    egui::Window::new("Edition panel")
        .title_bar(true)
        // TODO Init all those values with viewport size
        .default_pos(Pos2::new(10., 300.))
        .show(contexts.ctx_mut(), |ui| {
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
                            brush_events.send(BrushEvent::ClearBrush);
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
                            brush_events.send(BrushEvent::UpdateBrush(ModelBrush {
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
                                    brush_events.send(BrushEvent::UpdateRotation(*rotation));
                                }
                            }
                        }
                    });
                }
            });
        });
}

pub fn update_brush(
    mut editor_context: ResMut<EditorContext>,
    mut brush_events: EventReader<BrushEvent>,
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

pub fn update_painting_state(
    mut editor_context: ResMut<EditorContext>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut node_select_events: EventReader<NodeSelectedEvent>,
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

pub fn paint<C: CoordinateSystem>(
    mut editor_context: ResMut<EditorContext>,
    keys: Res<ButtonInput<KeyCode>>,
    active_generation: Res<ActiveGeneration>,
    mut node_over_events: EventReader<NodeOverEvent>,
    mut generations: Query<&mut Generator<C>>,
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
            warn!(
                "Failed to generate model {} on node {}: {}",
                model_brush.instance, node.0, err
            );
        }
    }
}

pub fn generate_node<C: CoordinateSystem>(
    active_generation: Entity,
    selected_node: &TargetedNode,
    model_index: ModelIndex,
    model_rot: ModelRotation,
    mut generations: &mut Query<&mut Generator<C>>,
) {
    let Ok(mut generator) = generations.get_mut(active_generation) else {
        return;
    };
    if let Err(err) =
        generator.set_and_propagate(selected_node.node_index, (model_index, model_rot), true)
    {
        warn!(
            "Failed to generate model {} rotation {} on node {}: {}",
            model_index, model_rot, selected_node.node_index, err
        );
    }
}
