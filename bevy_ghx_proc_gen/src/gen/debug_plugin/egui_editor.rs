use bevy::{
    ecs::{
        entity::Entity,
        query::With,
        system::{Local, Query, Res},
    },
    log::warn,
};
use bevy_egui::{
    egui::{self, Pos2},
    EguiContexts,
};
use ghx_proc_gen::{
    generator::{
        model::{ModelIndex, ModelRotation},
        Generator,
    },
    grid::direction::CoordinateSystem,
};

use super::{
    cursor::{Cursor, CursorInfo, SelectCursor, TargetedNode},
    generation::ActiveGeneration,
};

pub fn draw_edit_window<C: CoordinateSystem>(
    // TODO Default value may not be spawned on some nodes.
    mut local_selected_model_index: Local<Option<ModelIndex>>,
    mut contexts: EguiContexts,
    active_generation: Res<ActiveGeneration>,
    mut generations: Query<&mut Generator<C>>,
    selection_cursor: Query<(&Cursor, &CursorInfo), With<SelectCursor>>,
) {
    let Ok((cursor, cursor_info)) = selection_cursor.get_single() else {
        return;
    };
    let Some(selected_node) = &cursor.0 else {
        return;
    };
    let Some(active_generation) = active_generation.0 else {
        return;
    };
    if cursor_info.total_models_count <= 1 {
        return;
    }

    egui::Window::new("Edit")
        .title_bar(false)
        // TODO Init all those values with viewport size
        .default_pos(Pos2::new(10., 250.))
        .show(contexts.ctx_mut(), |ui| {
            ui.label(format!(
                "{} possible models ({} variations)",
                cursor_info.models_variations.len(),
                cursor_info.total_models_count,
            ));
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for model_group in cursor_info.models_variations.iter() {
                    let selected = local_selected_model_index.is_some()
                        && local_selected_model_index.unwrap() == model_group.index;
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
                            } else {
                                *local_selected_model_index = Some(model_group.index);
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
        });
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
        generator.set_and_propagate(selected_node.node_index, (model_index, model_rot))
    {
        warn!(
            "Failed to generate model {} rotation {} on node {}: {}",
            model_index, model_rot, selected_node.node_index, err
        );
    }
}
