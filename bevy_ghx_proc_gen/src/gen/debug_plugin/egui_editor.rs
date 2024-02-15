use bevy::{
    ecs::{
        entity::Entity,
        event::EventReader,
        query::With,
        system::{Query, Res, ResMut, Resource},
    },
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    log::{info, warn},
};
use bevy_egui::{
    egui::{self, Pos2},
    EguiContexts,
};
use ghx_proc_gen::{
    generator::{
        model::{ModelIndex, ModelInstance, ModelRotation},
        Generator,
    },
    grid::direction::CoordinateSystem,
};

use crate::gen::GridNode;

use super::{
    cursor::{Cursor, CursorInfo, SelectCursor, TargetedNode},
    generation::ActiveGeneration,
    picking::{CursorTarget, NodeOverEvent, NodeSelectedEvent},
};

#[derive(Resource, Default)]
pub struct EditorContext {
    pub selected_model: Option<ModelInstance>,
    pub paint_mode_enabled: bool,
    pub painting: bool,
}

pub fn draw_cursor_edit_window<C: CoordinateSystem>(
    mut editor_context: ResMut<EditorContext>,
    mut contexts: EguiContexts,
    active_generation: Res<ActiveGeneration>,
    mut generations: Query<&mut Generator<C>>,
    selection_cursor: Query<(&Cursor, &CursorInfo), With<SelectCursor>>,
) {
    let Some(active_generation) = active_generation.0 else {
        return;
    };

    egui::Window::new("Debug-editor")
        .title_bar(false)
        // TODO Init all those values with viewport size
        .default_pos(Pos2::new(10., 250.))
        .show(contexts.ctx_mut(), |ui| {
            ui.checkbox(&mut editor_context.paint_mode_enabled, "🖊 Painting");
            ui.separator();
            match editor_context.paint_mode_enabled {
                true => {
                    let Ok(generator) = generations.get(active_generation) else {
                        return;
                    };
                    ui.label(format!(
                        "rules: {} models ({} variations)",
                        generator.rules().original_models_count(),
                        generator.rules().models_count(),
                    ));
                    ui.separator();
                    // TODO Cache all this info ? original_models_count, models_count, rules models groups
                    //------ temporary
                    let Ok((_cursor, cursor_info)) = selection_cursor.get_single() else {
                        return;
                    };
                    ui.label(format!(
                        "selected: {} possible models ({} variations)",
                        cursor_info.models_variations.len(),
                        cursor_info.total_models_count,
                    ));
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for model_group in cursor_info.models_variations.iter() {
                            let selected = match editor_context.selected_model {
                                Some(instance) => model_group.index == instance.model_index,
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
                                        format!("{}{}", model_group.info.name, rot_count_tag,),
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
                                    editor_context.selected_model = Some(ModelInstance {
                                        model_index: model_group.index,
                                        rotation: model_group.rotations[0],
                                    });
                                }
                                if selected {
                                    for rotation in model_group.rotations.iter() {
                                        let is_selected = match editor_context.selected_model {
                                            Some(instance) => *rotation == instance.rotation,
                                            None => false,
                                        };
                                        if ui
                                            .selectable_label(
                                                is_selected,
                                                format!("{}°", rotation.value()),
                                            )
                                            .clicked()
                                        {
                                            editor_context
                                                .selected_model
                                                .as_mut()
                                                .unwrap()
                                                .rotation = *rotation;
                                        }
                                    }
                                }
                            });
                        }
                    });
                    //------ temporary
                }
                false => {
                    let Ok((cursor, cursor_info)) = selection_cursor.get_single() else {
                        return;
                    };
                    let Some(selected_node) = &cursor.0 else {
                        return;
                    };
                    if cursor_info.total_models_count <= 1 {
                        return;
                    }
                    ui.label(format!(
                        "selected: {} possible models ({} variations)",
                        cursor_info.models_variations.len(),
                        cursor_info.total_models_count,
                    ));
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for model_group in cursor_info.models_variations.iter() {
                            let selected = match editor_context.selected_model {
                                Some(instance) => model_group.index == instance.model_index,
                                None => false,
                            };
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        selected,
                                        format!(
                                            "[{}] {}",
                                            model_group.rotations.len(),
                                            model_group.info.name
                                        ),
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
                                    if model_group.rotations.len() == 1 {
                                        generate_node(
                                            active_generation,
                                            selected_node,
                                            model_group.index,
                                            model_group.rotations[0],
                                            &mut generations,
                                        );
                                        editor_context.selected_model = None;
                                    } else {
                                        //TODO Meh. May need to split model & rot again for cursor mode
                                        editor_context.selected_model = Some(ModelInstance {
                                            model_index: model_group.index,
                                            rotation: model_group.rotations[0],
                                        });
                                    }
                                }
                                if selected {
                                    for rot in model_group.rotations.iter() {
                                        if ui.button(format!("{}°", rot.value())).clicked() {
                                            generate_node(
                                                active_generation,
                                                selected_node,
                                                model_group.index,
                                                *rot,
                                                &mut generations,
                                            );
                                        }
                                    }
                                }
                            });
                        }
                    });
                }
            }
        });
}

pub fn update_painting_state(
    mut editor_context: ResMut<EditorContext>,
    buttons: Res<Input<MouseButton>>,
    mut node_select_events: EventReader<NodeSelectedEvent>,
    cursor_targets: Query<(), With<CursorTarget>>,
) {
    if !editor_context.paint_mode_enabled {
        node_select_events.clear();
        editor_context.painting = false;
        return;
    }
    if let Some(ev) = node_select_events.read().last() {
        if let Ok(_) = cursor_targets.get(ev.0) {
            editor_context.painting = true;
            info!(" editor_context.painting = true;",);
        };
    }
    if buttons.just_released(MouseButton::Left) {
        editor_context.painting = false;
        info!(" editor_context.painting = false;",);
    }
}

pub fn paint<C: CoordinateSystem>(
    mut editor_context: ResMut<EditorContext>,
    keys: Res<Input<KeyCode>>,
    active_generation: Res<ActiveGeneration>,
    mut node_over_events: EventReader<NodeOverEvent>,
    mut generations: Query<&mut Generator<C>>,
    cursor_targets: Query<&GridNode, With<CursorTarget>>,
) {
    if !editor_context.painting {
        node_over_events.clear();
        return;
    }
    let Some(paint_instance) = editor_context.selected_model else {
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

        info!(
            "Request to generate model {}  on node {}",
            paint_instance, node.0
        );
        if let Err(err) = generator.set_and_propagate(node.0, paint_instance, true) {
            warn!(
                "Failed to generate model {}  on node {}: {}",
                paint_instance, node.0, err
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
