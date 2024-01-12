use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, Update},
    asset::Asset,
    ecs::{
        bundle::Bundle,
        entity::Entity,
        event::EventWriter,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    input::{keyboard::KeyCode, Input},
    log::{info, warn},
    math::Vec3,
    render::color::Color,
    time::{Time, Timer, TimerMode},
};
use ghx_proc_gen::{
    generator::{model::ModelInstance, observer::GenerationUpdate, GenerationStatus},
    GenerationError,
};

use crate::{
    grid::{markers::MarkerEvent, SharableCoordSystem},
    Generation,
};

use super::{spawn_node, SpawnedNode};

pub struct ProcGenDebugPlugin<C: SharableCoordSystem, A: Asset, B: Bundle> {
    generation_view_mode: GenerationViewMode,
    typestate: PhantomData<(C, A, B)>,
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> ProcGenDebugPlugin<C, A, B> {
    pub fn new(generation_view_mode: GenerationViewMode) -> Self {
        Self {
            generation_view_mode,
            typestate: PhantomData,
        }
    }
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> Plugin for ProcGenDebugPlugin<C, A, B> {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.generation_view_mode);

        // If the resources already exists, nothing happens, else, add with default values.
        app.init_resource::<ProcGenKeyBindings>();
        app.init_resource::<GenerationControl>();

        app.add_systems(Update, update_generation_control);

        match self.generation_view_mode {
            GenerationViewMode::StepByStepTimed(steps, interval) => {
                app.add_systems(
                    Update,
                    (
                        step_by_step_timed_update::<C, A, B>,
                        update_generation_view::<C, A, B>,
                    )
                        .chain(),
                );
                app.insert_resource(StepByStepTimed {
                    steps,
                    timer: Timer::new(Duration::from_millis(interval), TimerMode::Repeating),
                });
            }
            GenerationViewMode::StepByStepPaused => {
                app.add_systems(
                    Update,
                    (
                        step_by_step_input_update::<C, A, B>,
                        update_generation_view::<C, A, B>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (generate_all::<C, A, B>, update_generation_view::<C, A, B>).chain(),
                );
            }
        }
    }
}

/// Controls how the generation occurs.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum GenerationViewMode {
    /// Generates step by step and waits at least the specified amount (in milliseconds) between each step.
    StepByStepTimed(u32, u64),
    /// Generates step by step and waits for a user input between each step.
    StepByStepPaused,
    /// Generates it all at once at the start
    Final,
}

// Read by the examples plugin when generating
#[derive(Resource)]
pub struct GenerationControl {
    status: GenerationControlStatus,
    /// Whether or not the spawning systems should skip over when nodes without assets are generated.
    pub skip_void_nodes: bool,
    /// Whether or not the generation should pause when successful
    pub pause_when_done: bool,
    /// Whether or not the generation should pause when it fails
    pub pause_on_error: bool,
}

impl Default for GenerationControl {
    fn default() -> Self {
        Self {
            status: GenerationControlStatus::Ongoing,
            skip_void_nodes: true,
            pause_when_done: true,
            pause_on_error: true,
        }
    }
}

impl GenerationControl {
    pub fn new(skip_void_nodes: bool, pause_when_done: bool, pause_on_error: bool) -> Self {
        Self {
            status: GenerationControlStatus::Ongoing,
            skip_void_nodes,
            pause_on_error,
            pause_when_done,
        }
    }
}

#[derive(Resource, Eq, PartialEq, Debug)]
pub enum GenerationControlStatus {
    Paused,
    Ongoing,
}

// Resource to track the generation steps when using [`GenerationViewMode::StepByStepTimed`]
#[derive(Resource)]
pub struct StepByStepTimed {
    pub steps: u32,
    pub timer: Timer,
}

#[derive(Resource)]
pub struct ProcGenKeyBindings {
    pub unpause: KeyCode,
    pub step: KeyCode,
    pub continuous_step: KeyCode,
}

impl Default for ProcGenKeyBindings {
    fn default() -> Self {
        Self {
            unpause: KeyCode::Space,
            step: KeyCode::Right,
            continuous_step: KeyCode::Up,
        }
    }
}

pub fn generate_all<C: SharableCoordSystem, A: Asset, B: Bundle>(
    mut generation_control: ResMut<GenerationControl>,
    mut generations: Query<&mut Generation<C, A, B>>,
) {
    for mut generation in generations.iter_mut() {
        if generation_control.status == GenerationControlStatus::Ongoing {
            match generation.gen.generate() {
                Ok(()) => {
                    info!(
                        "Generation done, seed: {}; grid: {}",
                        generation.gen.get_seed(),
                        generation.gen.grid()
                    );
                }
                Err(GenerationError { node_index }) => {
                    warn!(
                        "Generation Failed at node {}, seed: {}; grid: {}",
                        node_index,
                        generation.gen.get_seed(),
                        generation.gen.grid()
                    );
                }
            }
            generation_control.status = GenerationControlStatus::Paused;
        }
    }
}

pub fn update_generation_control(
    keys: Res<Input<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut generation_control: ResMut<GenerationControl>,
) {
    if keys.just_pressed(proc_gen_key_bindings.unpause) {
        match generation_control.status {
            GenerationControlStatus::Paused => {
                generation_control.status = GenerationControlStatus::Ongoing;
            }
            GenerationControlStatus::Ongoing => (),
        }
    }
}

pub fn step_by_step_input_update<C: SharableCoordSystem, A: Asset, B: Bundle>(
    keys: Res<Input<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut generation_control: ResMut<GenerationControl>,
    mut generations: Query<&mut Generation<C, A, B>>,
) {
    if generation_control.status == GenerationControlStatus::Ongoing
        && (keys.just_pressed(proc_gen_key_bindings.step)
            || keys.pressed(proc_gen_key_bindings.continuous_step))
    {
        for mut generation in generations.iter_mut() {
            step_generation(&mut generation, &mut generation_control);
        }
    }
}

pub fn step_by_step_timed_update<C: SharableCoordSystem, A: Asset, B: Bundle>(
    mut generation_control: ResMut<GenerationControl>,
    mut steps_and_timer: ResMut<StepByStepTimed>,
    time: Res<Time>,
    mut generations: Query<&mut Generation<C, A, B>>,
) {
    steps_and_timer.timer.tick(time.delta());
    if steps_and_timer.timer.finished()
        && generation_control.status == GenerationControlStatus::Ongoing
    {
        for mut generation in generations.iter_mut() {
            for _ in 0..steps_and_timer.steps {
                step_generation(&mut generation, &mut generation_control);
                if generation_control.status != GenerationControlStatus::Ongoing {
                    break;
                }
            }
        }
    }
}

fn update_generation_view<C: SharableCoordSystem, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut marker_events: EventWriter<MarkerEvent>,
    mut generators: Query<(Entity, &mut Generation<C, A, B>)>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    for (gen_entity, mut generation) in generators.iter_mut() {
        let mut reinitialized = false;
        let mut nodes_to_spawn = Vec::new();
        for update in generation.observer.dequeue_all() {
            match update {
                GenerationUpdate::Generated(grid_node) => {
                    nodes_to_spawn.push(grid_node);
                }
                GenerationUpdate::Reinitializing(_) => {
                    reinitialized = true;
                    nodes_to_spawn.clear();
                }
                GenerationUpdate::Failed(node_index) => {
                    marker_events.send(MarkerEvent::Add {
                        color: Color::RED,
                        grid_entity: gen_entity,
                        node_index,
                    });
                }
            }
        }

        if reinitialized {
            for existing_node in existing_nodes.iter() {
                commands.entity(existing_node).despawn_recursive();
            }
            marker_events.send(MarkerEvent::ClearAll);
        }

        for grid_node in nodes_to_spawn {
            spawn_node(
                &mut commands,
                gen_entity,
                &generation,
                &grid_node.model_instance,
                grid_node.node_index,
            );
        }
    }
}

fn step_generation<C: SharableCoordSystem, A: Asset, B: Bundle>(
    generation: &mut Generation<C, A, B>,
    generation_control: &mut ResMut<GenerationControl>,
) {
    loop {
        let mut non_void_spawned = false;
        match generation.gen.select_and_propagate_collected() {
            Ok((status, nodes_to_spawn)) => {
                for grid_node in nodes_to_spawn {
                    // We still collect the generated nodes here even though we don't really use them to spawn entities. We just check them for void nodes (for visualization purposes)
                    if let Some(assets) = generation
                        .models_assets
                        .get(&grid_node.model_instance.model_index)
                    {
                        if !assets.is_empty() {
                            non_void_spawned = true;
                        }
                    }
                }
                match status {
                    GenerationStatus::Ongoing => {}
                    GenerationStatus::Done => {
                        info!(
                            "Generation done, seed: {}; grid: {}",
                            generation.gen.get_seed(),
                            generation.gen.grid()
                        );
                        if generation_control.pause_when_done {
                            generation_control.status = GenerationControlStatus::Paused;
                        }
                        break;
                    }
                }
            }
            Err(GenerationError { node_index }) => {
                warn!(
                    "Generation Failed at node {}, seed: {}; grid: {}",
                    node_index,
                    generation.gen.get_seed(),
                    generation.gen.grid()
                );
                if generation_control.pause_on_error {
                    generation_control.status = GenerationControlStatus::Paused;
                }
                break;
            }
        }

        // If we want to skip over void nodes, we eep looping until we spawn a non-void
        if non_void_spawned | !generation_control.skip_void_nodes {
            break;
        }
    }
}
