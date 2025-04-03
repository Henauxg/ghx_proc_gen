use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, Update},
    color::{palettes::css::RED, Color},
    ecs::{
        entity::Entity,
        event::EventWriter,
        query::{With, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    input::{keyboard::KeyCode, ButtonInput},
    log::{info, warn},
    prelude::{Component, Deref, DerefMut, Resource},
    time::{Time, Timer, TimerMode},
};
use bevy_ghx_grid::debug_plugin::markers::{spawn_marker, MarkerDespawnEvent};
use ghx_proc_gen::{
    generator::{
        observer::{GenerationUpdate, QueuedObserver},
        GenerationStatus, Generator,
    },
    ghx_grid::cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid},
    GeneratorError, NodeIndex,
};

use crate::{GenerationResetEvent, NodesGeneratedEvent, VoidNodes};

use super::{DebugPluginConfig, ProcGenKeyBindings};

/// Picking plugin for the [super::ProcGenDebugRunnerPlugin]
#[derive(Default)]
pub(crate) struct ProcGenDebugGenerationPlugin<C: CartesianCoordinates> {
    /// Used to configure how the cursors UI should be displayed
    pub config: DebugPluginConfig,
    #[doc(hidden)]
    pub typestate: PhantomData<C>,
}

impl<C: CartesianCoordinates> Plugin for ProcGenDebugGenerationPlugin<C> {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.generation_view_mode)
            .insert_resource(ActiveGeneration::default())
            .init_resource::<GenerationControl>();

        app.add_systems(
            Update,
            (update_generation_control, update_active_generation::<C>),
        );

        match self.config.generation_view_mode {
            GenerationViewMode::StepByStepTimed {
                steps_count,
                interval_ms,
            } => {
                app.add_systems(
                    Update,
                    (
                        (insert_error_markers_to_new_generations::<C>,),
                        step_by_step_timed_update::<C>,
                        dequeue_generation_updates::<C>,
                    )
                        .chain(),
                );
                app.insert_resource(StepByStepTimed {
                    steps_count,
                    timer: Timer::new(Duration::from_millis(interval_ms), TimerMode::Repeating),
                });
            }
            GenerationViewMode::StepByStepManual => {
                app.add_systems(
                    Update,
                    (
                        (insert_error_markers_to_new_generations::<C>,),
                        step_by_step_input_update::<C>,
                        dequeue_generation_updates::<C>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (generate_all::<C>, dequeue_generation_updates::<C>).chain(),
                );
            }
        }
    }
}

impl<C: CartesianCoordinates> ProcGenDebugGenerationPlugin<C> {
    /// Constructor
    pub fn new(config: &DebugPluginConfig) -> Self {
        Self {
            config: config.clone(),
            ..Default::default()
        }
    }
}

/// Controls how the generation occurs.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationViewMode {
    /// Generates steps by steps and waits at least the specified amount (in milliseconds) between each step.
    StepByStepTimed {
        /// How many steps to run once the timer has finished a cycle
        steps_count: u32,
        /// Time to wait in ms before the next steps
        interval_ms: u64,
    },
    /// Generates step by step and waits for a user input between each step.
    StepByStepManual,
    /// Generates it all at once at the start
    #[default]
    Final,
}

/// Used to track the status of the generation control
#[derive(Eq, PartialEq, Debug)]
pub enum GenerationControlStatus {
    /// Generation control is paused, systems won't automatically step the generation
    Paused,
    /// Generation control is "ongoing", systems can currently step a generator
    Ongoing,
}

/// Read by the systems while generating
#[derive(Resource)]
pub struct GenerationControl {
    /// Current status of the generation
    pub status: GenerationControlStatus,
    /// Indicates whether or not the generator needs to be reinitialized before calling generation operations.
    ///
    /// When using [`GenerationViewMode::Final`], this only controls the first reinitialization per try pool.
    pub need_reinit: bool,
    /// Whether or not the spawning systems do one more generation step when nodes without assets are generated.
    ///
    /// Not used when using [`GenerationViewMode::Final`].
    pub skip_void_nodes: bool,
    /// Whether or not the generation should pause when successful
    pub pause_when_done: bool,
    /// Whether or not the generation should pause when it fails.
    ///
    /// When using [`GenerationViewMode::Final`], this only pauses on the last error of a try pool.
    pub pause_on_error: bool,
    /// Whether or not the generation should pause when it reinitializes
    ///
    /// When using [`GenerationViewMode::Final`], this only pauses on the first reinitialization of a try pool.
    pub pause_on_reinitialize: bool,
}

impl Default for GenerationControl {
    fn default() -> Self {
        Self {
            status: GenerationControlStatus::Paused,
            need_reinit: false,
            skip_void_nodes: true,
            pause_when_done: true,
            pause_on_error: true,
            pause_on_reinitialize: true,
        }
    }
}

/// Resource to track the generation steps when using [`GenerationViewMode::StepByStepTimed`]
#[derive(Resource)]
pub struct StepByStepTimed {
    /// How many steps should be done once the timer has expired
    pub steps_count: u32,
    /// Timer, tracking the time between the steps
    pub timer: Timer,
}

/// Component used to store a collection of [`bevy_ghx_grid::debug_plugin::markers::GridMarker`] entities
#[derive(Component, Default, Deref, DerefMut)]
pub struct ErrorMarkers(pub Vec<Entity>);

/// Resource used to track the currently active generation.
///
/// The contained option can be [None] if no generation is active
#[derive(Resource, Default)]
pub struct ActiveGeneration(pub Option<Entity>);

/// System used to insert an empty [ErrorMarkers] component into new generation entities
pub fn insert_error_markers_to_new_generations<C: CartesianCoordinates>(
    mut commands: Commands,
    mut new_generations: Query<
        Entity,
        (With<Generator<C, CartesianGrid<C>>>, Without<ErrorMarkers>),
    >,
) {
    for gen_entity in new_generations.iter_mut() {
        commands.entity(gen_entity).insert(ErrorMarkers::default());
    }
}

/// System that will update the currenty active generation if it was [None]
pub fn update_active_generation<C: CartesianCoordinates>(
    mut active_generation: ResMut<ActiveGeneration>,
    generations: Query<Entity, With<Generator<C, CartesianGrid<C>>>>,
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
    keys: Res<ButtonInput<KeyCode>>,
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
pub fn handle_reinitialization_and_continue<C: CartesianCoordinates>(
    generation_control: &mut ResMut<GenerationControl>,
    generator: &mut Generator<C, CartesianGrid<C>>,
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

/// Function used to display some info about a generation that finished,
/// as well as to properly handle reinitialization status and pause.
pub fn handle_generation_done<C: CartesianCoordinates>(
    generation_control: &mut ResMut<GenerationControl>,
    generator: &mut Generator<C, CartesianGrid<C>>,
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

/// Function used to display some info about a generation that failed,
/// as well as to properly handle reinitialization status and pause.
pub fn handle_generation_error<C: CartesianCoordinates>(
    generation_control: &mut ResMut<GenerationControl>,
    generator: &mut Generator<C, CartesianGrid<C>>,
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
pub fn generate_all<C: CartesianCoordinates>(
    mut generation_control: ResMut<GenerationControl>,
    active_generation: Res<ActiveGeneration>,
    mut observed_generatiors: Query<&mut Generator<C, CartesianGrid<C>>, With<QueuedObserver>>,
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
pub fn step_by_step_input_update<C: CartesianCoordinates>(
    keys: Res<ButtonInput<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut generation_control: ResMut<GenerationControl>,
    active_generation: Res<ActiveGeneration>,
    mut observed_generations: Query<
        (&mut Generator<C, CartesianGrid<C>>, Option<&VoidNodes>),
        With<QueuedObserver>,
    >,
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
pub fn step_by_step_timed_update<C: CartesianCoordinates>(
    mut generation_control: ResMut<GenerationControl>,
    mut steps_and_timer: ResMut<StepByStepTimed>,
    time: Res<Time>,
    active_generation: Res<ActiveGeneration>,
    mut observed_generations: Query<
        (&mut Generator<C, CartesianGrid<C>>, Option<&VoidNodes>),
        With<QueuedObserver>,
    >,
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

/// System used to spawn nodes, emit [GenerationResetEvent] & [NodesGeneratedEvent] and despawn markers, based on data read from a [QueuedObserver] on a generation entity
pub fn dequeue_generation_updates<C: CartesianCoordinates>(
    mut commands: Commands,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut generators: Query<(
        Entity,
        &CartesianGrid<C>,
        &mut QueuedObserver,
        Option<&mut ErrorMarkers>,
    )>,
) {
    for (grid_entity, grid, mut observer, mut error_markers) in generators.iter_mut() {
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
                            Color::Srgba(RED),
                            grid.pos_from_index(node_index),
                        ));
                    }
                }
            }
        }

        if reinitialized {
            commands.trigger_targets(GenerationResetEvent, grid_entity);
            if let Some(error_markers) = error_markers.as_mut() {
                for marker in error_markers.iter() {
                    marker_events.send(MarkerDespawnEvent::Marker(*marker));
                }
                error_markers.clear();
            }
        }

        if !nodes_to_spawn.is_empty() {
            commands.trigger_targets(NodesGeneratedEvent(nodes_to_spawn), grid_entity);
        }
    }
}

fn step_generation<C: CartesianCoordinates>(
    generator: &mut Generator<C, CartesianGrid<C>>,
    gen_entity: Entity,
    void_nodes: Option<&VoidNodes>,
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
                    non_void_spawned = match void_nodes {
                        Some(void_nodes) => {
                            !void_nodes.contains(&grid_node.model_instance.model_index)
                        }
                        None => true,
                    };
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
