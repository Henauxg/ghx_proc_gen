use std::collections::HashSet;

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventWriter},
        query::{With, Without},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::{Children, DespawnRecursiveExt},
    input::{keyboard::KeyCode, Input},
    log::{info, warn},
    prelude::{Deref, DerefMut},
    render::color::Color,
    time::Time,
};
use bevy_ghx_grid::{
    debug_plugin::markers::{spawn_marker, MarkerDespawnEvent},
    ghx_grid::{coordinate_system::CoordinateSystem, grid::GridDefinition},
};
use ghx_proc_gen::{
    generator::{
        model::ModelIndex,
        observer::{GenerationUpdate, QueuedObserver},
        GenerationStatus, Generator,
    },
    GeneratorError, NodeIndex,
};

use crate::gen::SpawnedNode;

use super::{
    spawn_node, AssetSpawner, AssetsBundleSpawner, ComponentSpawner, GenerationControl,
    GenerationControlStatus, ProcGenKeyBindings, StepByStepTimed,
};

/// Component used to store model indexes of models with no assets, just to be able to skip their generation when stepping
#[derive(Component, Default, Deref, DerefMut)]
pub struct VoidNodes(pub HashSet<ModelIndex>);

#[derive(Component, Default, Deref, DerefMut)]
pub struct ErrorMarkers(pub Vec<Entity>);

#[derive(Event, Clone, Copy, Debug)]
pub enum GenerationEvent {
    Reinitialized(Entity),
    Updated(Entity, NodeIndex),
}

#[derive(Resource, Default)]
pub struct ActiveGeneration(pub Option<Entity>);

/// Simple system that calculates and add a [`VoidNodes`] component for generator entites which don't have one yet.
pub fn insert_void_nodes_to_new_generations<
    C: CoordinateSystem,
    A: AssetsBundleSpawner,
    T: ComponentSpawner,
>(
    mut commands: Commands,
    mut new_generations: Query<
        (Entity, &mut Generator<C>, &AssetSpawner<A, T>),
        Without<VoidNodes>,
    >,
) {
    for (gen_entity, generation, asset_spawner) in new_generations.iter_mut() {
        let mut void_nodes = HashSet::new();
        for model_index in 0..generation.rules().original_models_count() {
            if !asset_spawner.assets.contains_key(&model_index) {
                void_nodes.insert(model_index);
            }
        }
        commands.entity(gen_entity).insert(VoidNodes(void_nodes));
    }
}

pub fn insert_error_markers_to_new_generations<C: CoordinateSystem>(
    mut commands: Commands,
    mut new_generations: Query<Entity, (With<Generator<C>>, Without<ErrorMarkers>)>,
) {
    for gen_entity in new_generations.iter_mut() {
        commands.entity(gen_entity).insert(ErrorMarkers::default());
    }
}

pub fn update_active_generation<C: CoordinateSystem>(
    mut active_generation: ResMut<ActiveGeneration>,
    generations: Query<Entity, With<Generator<C>>>,
) {
    if active_generation.0.is_some() {
        return;
    }

    if let Some(gen_entity) = generations.iter().last() {
        active_generation.0 = Some(gen_entity);
    }
}

/// This system pauses/unpauses the [`GenerationControlStatus`] in the [`GenerationControl`] `Resource` on a keypress.
///
/// The keybind is read from the [`ProcGenKeyBindings`] `Resource`
pub fn update_generation_control(
    keys: Res<Input<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut generation_control: ResMut<GenerationControl>,
) {
    if keys.just_pressed(proc_gen_key_bindings.pause_toggle) {
        generation_control.status = match generation_control.status {
            GenerationControlStatus::Ongoing => GenerationControlStatus::Paused,
            GenerationControlStatus::Paused => GenerationControlStatus::Ongoing,
        };
    }
}

/// - reinitializes the generator if needed
/// - returns `true` if the generation operation should continue, and `false` if it should stop
pub fn handle_reinitialization_and_continue<C: CoordinateSystem>(
    generation_control: &mut ResMut<GenerationControl>,
    generator: &mut Generator<C>,
) -> bool {
    if generation_control.need_reinit {
        generation_control.need_reinit = false;
        match generator.reinitialize() {
            GenerationStatus::Ongoing => (),
            GenerationStatus::Done => {
                info!(
                    "Generation done, seed: {}; grid: {}",
                    generator.seed(),
                    generator.grid()
                );
                if generation_control.pause_when_done {
                    generation_control.status = GenerationControlStatus::Paused;
                }
                generation_control.need_reinit = true;
                return false;
            }
        }
        if generation_control.pause_on_reinitialize {
            generation_control.status = GenerationControlStatus::Paused;
            return false;
        }
    }
    return true;
}

pub fn handle_generation_done<C: CoordinateSystem>(
    generation_control: &mut ResMut<GenerationControl>,
    generator: &mut Generator<C>,
    gen_entity: Entity,
    try_count: u32,
) {
    info!(
        "Generation done {:?}, try_count: {}, seed: {}; grid: {}",
        gen_entity,
        try_count,
        generator.seed(),
        generator.grid()
    );
    generation_control.need_reinit = true;
    if generation_control.pause_when_done {
        generation_control.status = GenerationControlStatus::Paused;
    }
}

pub fn handle_generation_error<C: CoordinateSystem>(
    generation_control: &mut ResMut<GenerationControl>,
    generator: &mut Generator<C>,
    gen_entity: Entity,
    node_index: NodeIndex,
) {
    warn!(
        "Generation Failed {:?} at node {}, seed: {}; grid: {}",
        gen_entity,
        node_index,
        generator.seed(),
        generator.grid()
    );
    generation_control.need_reinit = true;
    if generation_control.pause_on_error {
        generation_control.status = GenerationControlStatus::Paused;
    }
}

/// This system request the full generation to a [`Generator`] component, if it is observed through a [`QueuedObserver`] component, if the current control status is [`GenerationControlStatus::Ongoing`] and if it is currently the [`ActiveGeneration`]
pub fn generate_all<C: CoordinateSystem>(
    mut generation_control: ResMut<GenerationControl>,
    active_generation: Res<ActiveGeneration>,
    mut observed_generatiors: Query<&mut Generator<C>, With<QueuedObserver>>,
) {
    let Some(active_generation) = active_generation.0 else {
        return;
    };
    let Ok(mut generator) = observed_generatiors.get_mut(active_generation) else {
        return;
    };

    if generation_control.status == GenerationControlStatus::Ongoing {
        if !handle_reinitialization_and_continue(&mut generation_control, &mut generator) {
            return;
        }

        match generator.generate() {
            Ok(gen_info) => {
                handle_generation_done(
                    &mut generation_control,
                    &mut generator,
                    active_generation,
                    gen_info.try_count,
                );
            }
            Err(GeneratorError { node_index }) => {
                handle_generation_error(
                    &mut generation_control,
                    &mut generator,
                    active_generation,
                    node_index,
                );
            }
        }
    }
}

/// This system steps a [`Generator`] component if it is  observed through a [`QueuedObserver`] component, if the current control status is [`GenerationControlStatus::Ongoing`], if it is currently the [`ActiveGeneration`] and if the appropriate keys are pressed.
///
/// The keybinds are read from the [`ProcGenKeyBindings`] `Resource`
pub fn step_by_step_input_update<C: CoordinateSystem>(
    keys: Res<Input<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut generation_control: ResMut<GenerationControl>,
    active_generation: Res<ActiveGeneration>,
    mut observed_generations: Query<(&mut Generator<C>, &VoidNodes), With<QueuedObserver>>,
) {
    let Some(active_generation) = active_generation.0 else {
        return;
    };

    if generation_control.status == GenerationControlStatus::Ongoing
        && (keys.just_pressed(proc_gen_key_bindings.step)
            || keys.pressed(proc_gen_key_bindings.continuous_step))
    {
        if let Ok((mut generation, void_nodes)) = observed_generations.get_mut(active_generation) {
            step_generation(
                &mut generation,
                active_generation,
                void_nodes,
                &mut generation_control,
            );
        }
    }
}

/// This system steps a [`Generator`] component if it is observed through a [`QueuedObserver`] component, if the current control status is [`GenerationControlStatus::Ongoing`] if it is currently the [`ActiveGeneration`] and if the timer in the [`StepByStepTimed`] `Resource` has finished.
pub fn step_by_step_timed_update<C: CoordinateSystem>(
    mut generation_control: ResMut<GenerationControl>,
    mut steps_and_timer: ResMut<StepByStepTimed>,
    time: Res<Time>,
    active_generation: Res<ActiveGeneration>,
    mut observed_generations: Query<(&mut Generator<C>, &VoidNodes), With<QueuedObserver>>,
) {
    let Some(active_generation) = active_generation.0 else {
        return;
    };

    steps_and_timer.timer.tick(time.delta());
    if steps_and_timer.timer.finished()
        && generation_control.status == GenerationControlStatus::Ongoing
    {
        if let Ok((mut generation, void_nodes)) = observed_generations.get_mut(active_generation) {
            for _ in 0..steps_and_timer.steps_count {
                step_generation(
                    &mut generation,
                    active_generation,
                    void_nodes,
                    &mut generation_control,
                );
                if generation_control.status != GenerationControlStatus::Ongoing {
                    return;
                }
            }
        }
    }
}

pub fn update_generation_view<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner>(
    mut commands: Commands,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut generation_events: EventWriter<GenerationEvent>,
    mut generators: Query<(
        Entity,
        &GridDefinition<C>,
        &AssetSpawner<A, T>,
        &mut QueuedObserver,
        Option<&Children>,
        Option<&mut ErrorMarkers>,
    )>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    for (grid_entity, grid, asset_spawner, mut observer, children, mut error_markers) in
        generators.iter_mut()
    {
        let mut reinitialized = false;
        let mut nodes_to_spawn = Vec::new();
        for update in observer.dequeue_all() {
            match update {
                GenerationUpdate::Generated(grid_node) => {
                    nodes_to_spawn.push(grid_node);
                }
                GenerationUpdate::Reinitializing(_) => {
                    reinitialized = true;
                    nodes_to_spawn.clear();
                }
                GenerationUpdate::Failed(node_index) => {
                    if let Some(error_markers) = error_markers.as_mut() {
                        error_markers.push(spawn_marker(
                            &mut commands,
                            grid_entity,
                            Color::RED,
                            grid.pos_from_index(node_index),
                        ));
                    }
                }
            }
        }

        if reinitialized {
            generation_events.send(GenerationEvent::Reinitialized(grid_entity));
            if let Some(children) = children {
                for &child in children.iter() {
                    if let Ok(node) = existing_nodes.get(child) {
                        commands.entity(node).despawn_recursive();
                    }
                }
            }

            if let Some(error_markers) = error_markers.as_mut() {
                for marker in error_markers.iter() {
                    marker_events.send(MarkerDespawnEvent::Marker(*marker));
                }
                error_markers.clear();
            }
        }

        for grid_node in nodes_to_spawn {
            generation_events.send(GenerationEvent::Updated(grid_entity, grid_node.node_index));

            spawn_node(
                &mut commands,
                grid_entity,
                &grid,
                asset_spawner,
                &grid_node.model_instance,
                grid_node.node_index,
            );
        }
    }
}

fn step_generation<C: CoordinateSystem>(
    generator: &mut Generator<C>,
    gen_entity: Entity,
    void_nodes: &VoidNodes,
    generation_control: &mut ResMut<GenerationControl>,
) {
    loop {
        if !handle_reinitialization_and_continue(generation_control, generator) {
            break;
        }

        let mut non_void_spawned = false;
        match generator.select_and_propagate_collected() {
            Ok((status, nodes_to_spawn)) => {
                for grid_node in nodes_to_spawn {
                    // We still collect the generated nodes here even though we don't really use them to spawn entities. We just check them for void nodes (for visualization purposes)
                    if !void_nodes.contains(&grid_node.model_instance.model_index) {
                        non_void_spawned = true;
                    }
                }
                match status {
                    GenerationStatus::Ongoing => {}
                    GenerationStatus::Done => {
                        handle_generation_done(generation_control, generator, gen_entity, 1);
                        break;
                    }
                }
            }
            Err(GeneratorError { node_index }) => {
                handle_generation_error(generation_control, generator, gen_entity, node_index);
                break;
            }
        }

        // If we want to skip over void nodes, we keep looping until we spawn a non-void
        if non_void_spawned | !generation_control.skip_void_nodes {
            break;
        }
    }
}
