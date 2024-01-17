use std::{collections::HashSet, marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventWriter,
        query::{With, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::DespawnRecursiveExt,
    input::{keyboard::KeyCode, Input},
    log::{info, warn},
    prelude::Deref,
    render::color::Color,
    time::{Time, Timer, TimerMode},
};
use ghx_proc_gen::{
    generator::{
        model::ModelIndex,
        observer::{GenerationUpdate, QueuedObserver},
        GenerationStatus, Generator,
    },
    grid::{direction::CoordinateSystem, GridDefinition},
    GenerationError,
};

use crate::grid::markers::MarkerEvent;

use super::{
    assets::NoComponents, spawn_node, AssetSpawner, AssetsBundleSpawner, ComponentSpawner,
    SpawnedNode,
};

/// A [`Plugin`] useful for debug/analysis/demo.
///
/// It takes in a [`GenerationViewMode`] to control how the generators in the [`Generator`] components will be run.
pub struct ProcGenDebugPlugin<
    C: CoordinateSystem,
    A: AssetsBundleSpawner,
    T: ComponentSpawner = NoComponents,
> {
    generation_view_mode: GenerationViewMode,
    typestate: PhantomData<(C, A, T)>,
}

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> ProcGenDebugPlugin<C, A, T> {
    /// Plugin constructor
    pub fn new(generation_view_mode: GenerationViewMode) -> Self {
        Self {
            generation_view_mode,
            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> Plugin
    for ProcGenDebugPlugin<C, A, T>
{
    fn build(&self, app: &mut App) {
        app.insert_resource(self.generation_view_mode);

        // If the resources already exists, nothing happens, else, add them with default values.
        app.init_resource::<ProcGenKeyBindings>();
        app.init_resource::<GenerationControl>();

        app.add_systems(Update, update_generation_control);

        match self.generation_view_mode {
            GenerationViewMode::StepByStepTimed {
                steps_count,
                interval_ms,
            } => {
                app.add_systems(
                    Update,
                    (
                        register_void_nodes_for_new_generations::<C, A, T>,
                        observe_new_generations::<C>,
                        step_by_step_timed_update::<C>,
                        update_generation_view::<C, A, T>,
                    )
                        .chain(),
                );
                app.insert_resource(StepByStepTimed {
                    steps_count,
                    timer: Timer::new(Duration::from_millis(interval_ms), TimerMode::Repeating),
                });
            }
            GenerationViewMode::StepByStepPaused => {
                app.add_systems(
                    Update,
                    (
                        register_void_nodes_for_new_generations::<C, A, T>,
                        observe_new_generations::<C>,
                        step_by_step_input_update::<C>,
                        update_generation_view::<C, A, T>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (
                        observe_new_generations::<C>,
                        generate_all::<C>,
                        update_generation_view::<C, A, T>,
                    )
                        .chain(),
                );
            }
        }
    }
}

/// Controls how the generation occurs.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum GenerationViewMode {
    /// Generates steps by steps and waits at least the specified amount (in milliseconds) between each step.
    StepByStepTimed {
        /// How many steps to run once the timer has finished a cycle
        steps_count: u32,
        /// Time to wait in ms before the next steps
        interval_ms: u64,
    },
    /// Generates step by step and waits for a user input between each step.
    StepByStepPaused,
    /// Generates it all at once at the start
    Final,
}

/// Used to track the status of the generation control
#[derive(Resource, Eq, PartialEq, Debug)]
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
    /// Whether or not the spawning systems do one more generation step when nodes without assets are generated.
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
    /// Create a new `GenerationControl` with the status set to [`GenerationControlStatus::Ongoing`]
    pub fn new(skip_void_nodes: bool, pause_when_done: bool, pause_on_error: bool) -> Self {
        Self {
            status: GenerationControlStatus::Ongoing,
            skip_void_nodes,
            pause_on_error,
            pause_when_done,
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

/// Resource available to override the default keybindings used by the [`ProcGenDebugPlugin`]
#[derive(Resource)]
pub struct ProcGenKeyBindings {
    /// Key to unpause the current [`GenerationControlStatus`]
    pub unpause: KeyCode,
    /// Key used only with [`GenerationViewMode::StepByStepPaused`] to step once per press
    pub step: KeyCode,
    /// Key used only with [`GenerationViewMode::StepByStepPaused`] to step continuously as long as pressed
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

/// Component added by the [`ProcGenDebugPlugin`] to entities with a [`Generator`] component. Used to analyze the generation process.
#[derive(Component)]
pub struct Observed {
    /// Generator observer
    pub obs: QueuedObserver,
}
impl Observed {
    fn new<C: CoordinateSystem>(mut generation: &mut Generator<C>) -> Self {
        Self {
            obs: QueuedObserver::new(&mut generation),
        }
    }
}

/// This system adds an [`Observed`] component to every `Entity` with a [`Generator`] component
pub fn observe_new_generations<C: CoordinateSystem>(
    mut commands: Commands,
    mut new_generations: Query<(Entity, &mut Generator<C>), Without<Observed>>,
) {
    for (gen_entity, mut generation) in new_generations.iter_mut() {
        commands
            .entity(gen_entity)
            .insert(Observed::new(&mut generation));
    }
}

/// Component used to store model indexes of models with no assets, just to be able to skip their generation when stepping
#[derive(Component, Deref)]
pub struct VoidNodes(HashSet<ModelIndex>);

/// Simple system that calculates and add a [`VoidNodes`] component for generator entites which don't have one yet.
pub fn register_void_nodes_for_new_generations<
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

/// This system unpauses the [`GenerationControlStatus`] in the [`GenerationControl`] `Resource` on a keypress.
///
/// The keybind is read from the [`ProcGenKeyBindings`] `Resource`
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

/// This system request the full generation to all [`Generator`] components, if they already are observed through an [`Observed`] component and if the current control status is [`GenerationControlStatus::Ongoing`]
pub fn generate_all<C: CoordinateSystem>(
    mut generation_control: ResMut<GenerationControl>,
    mut observed_generations: Query<&mut Generator<C>, With<Observed>>,
) {
    for mut generation in observed_generations.iter_mut() {
        if generation_control.status == GenerationControlStatus::Ongoing {
            match generation.generate() {
                Ok(()) => {
                    info!(
                        "Generation done, seed: {}; grid: {}",
                        generation.get_seed(),
                        generation.grid()
                    );
                }
                Err(GenerationError { node_index }) => {
                    warn!(
                        "Generation Failed at node {}, seed: {}; grid: {}",
                        node_index,
                        generation.get_seed(),
                        generation.grid()
                    );
                }
            }
            generation_control.status = GenerationControlStatus::Paused;
        }
    }
}

/// This system steps all [`Generator`] components if they already are observed through an [`Observed`] component, if the current control status is [`GenerationControlStatus::Ongoing`] and if the appropriate keys are pressed.
///
/// The keybinds are read from the [`ProcGenKeyBindings`] `Resource`
pub fn step_by_step_input_update<C: CoordinateSystem>(
    keys: Res<Input<KeyCode>>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut generation_control: ResMut<GenerationControl>,
    mut observed_generations: Query<(&mut Generator<C>, &VoidNodes), With<Observed>>,
) {
    if generation_control.status == GenerationControlStatus::Ongoing
        && (keys.just_pressed(proc_gen_key_bindings.step)
            || keys.pressed(proc_gen_key_bindings.continuous_step))
    {
        for (mut generation, void_nodes) in observed_generations.iter_mut() {
            step_generation(&mut generation, void_nodes, &mut generation_control);
        }
    }
}

/// This system steps all [`Generator`] components if they already are observed through an [`Observed`] component, if the current control status is [`GenerationControlStatus::Ongoing`] and if the timer in the [`StepByStepTimed`] `Resource` has finished.
pub fn step_by_step_timed_update<C: CoordinateSystem>(
    mut generation_control: ResMut<GenerationControl>,
    mut steps_and_timer: ResMut<StepByStepTimed>,
    time: Res<Time>,
    mut observed_generations: Query<(&mut Generator<C>, &VoidNodes), With<Observed>>,
) {
    steps_and_timer.timer.tick(time.delta());
    if steps_and_timer.timer.finished()
        && generation_control.status == GenerationControlStatus::Ongoing
    {
        for (mut generation, void_nodes) in observed_generations.iter_mut() {
            for _ in 0..steps_and_timer.steps_count {
                step_generation(&mut generation, void_nodes, &mut generation_control);
                if generation_control.status != GenerationControlStatus::Ongoing {
                    break;
                }
            }
        }
    }
}

fn update_generation_view<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner>(
    mut commands: Commands,
    mut marker_events: EventWriter<MarkerEvent>,
    mut generators: Query<(
        Entity,
        &GridDefinition<C>,
        &AssetSpawner<A, T>,
        &mut Observed,
    )>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    for (gen_entity, grid, asset_spawner, mut observer) in generators.iter_mut() {
        let mut reinitialized = false;
        let mut nodes_to_spawn = Vec::new();
        for update in observer.obs.dequeue_all() {
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
                &grid,
                asset_spawner,
                &grid_node.model_instance,
                grid_node.node_index,
            );
        }
    }
}

fn step_generation<C: CoordinateSystem>(
    generation: &mut Generator<C>,
    void_nodes: &VoidNodes,
    generation_control: &mut ResMut<GenerationControl>,
) {
    loop {
        let mut non_void_spawned = false;
        match generation.select_and_propagate_collected() {
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
                        info!(
                            "Generation done, seed: {}; grid: {}",
                            generation.get_seed(),
                            generation.grid()
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
                    generation.get_seed(),
                    generation.grid()
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
