use std::{
    collections::HashSet,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};

use bevy::{
    app::{App, Plugin, PostUpdate, PreUpdate, Startup, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventWriter,
        query::{Changed, With, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    input::{keyboard::KeyCode, Input},
    log::{info, warn},
    prelude::{Deref, DerefMut},
    render::color::Color,
    text::{Text, TextSection, TextStyle},
    time::{Time, Timer, TimerMode},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        BackgroundColor, PositionType, Style, UiRect, Val,
    },
    utils::default,
};
use ghx_proc_gen::{
    generator::{
        model::ModelIndex,
        observer::{GenerationUpdate, QueuedObserver},
        rules::ModelInfo,
        GenerationStatus, Generator,
    },
    grid::{
        direction::{CoordinateSystem, Direction},
        GridDefinition, GridPosition, NodeIndex,
    },
    GeneratorError,
};

#[cfg(feature = "picking")]
use bevy::{
    ecs::{
        event::{Event, EventReader},
        query::Added,
    },
    hierarchy::Parent,
};
#[cfg(feature = "picking")]
use bevy_mod_picking::{
    prelude::{Down, ListenerInput, On, Over, Pointer},
    PickableBundle,
};

#[cfg(feature = "picking")]
use super::insert_default_bundle_to_spawned_nodes;

use crate::grid::markers::{spawn_marker, GridMarker, MarkerDespawnEvent};

use super::{
    assets::NoComponents, spawn_node, AssetSpawner, AssetsBundleSpawner, ComponentSpawner,
    SpawnedNode,
};

const CURSOR_KEYS_MOVEMENT_COOLDOWN_MS: u64 = 55;

/// A [`Plugin`] useful for debug/analysis/demo. It mainly run [`Generator`] components and spawn the generated model's [`crate::gen::assets::ModelAsset`]
///
/// It takes in a [`GenerationViewMode`] to control how the generators components will be run.
///
/// It also uses the following `Resources`: [`ProcGenKeyBindings`] and [`GenerationControl`] (and will init them to their defaults if not inserted by the user).
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

        app.insert_resource(CursorMoveCooldown(Timer::new(
            Duration::from_millis(CURSOR_KEYS_MOVEMENT_COOLDOWN_MS),
            TimerMode::Once,
        )));

        #[cfg(feature = "picking")]
        app.add_event::<NodeOverEvent>()
            .add_event::<NodeSelectedEvent>();

        app.add_systems(Startup, setup_selection_cursor_info_ui);
        app.add_systems(
            Update,
            (
                update_generation_control,
                insert_selection_cursor_to_new_generations::<C>,
            ),
        );

        #[cfg(feature = "picking")]
        app.add_systems(
            Update,
            (
                insert_over_cursor_to_new_generations::<C>,
                insert_grid_cursor_picking_handlers_to_spawned_nodes::<C>,
                insert_default_bundle_to_spawned_nodes::<PickableBundle>,
            ),
        );
        // Keybinds and picking events handlers run in PreUpdate
        app.add_systems(
            PreUpdate,
            keybinds_update_grid_selection_cursor_position::<C>,
        );
        #[cfg(feature = "picking")]
        app.add_systems(
            Update,
            (
                picking_update_grid_cursor_position::<C, GridOverCursor, NodeOverEvent>,
                picking_update_grid_cursor_position::<C, GridSelectionCursor, NodeSelectedEvent>,
                update_grid_cursor_info_on_changes::<C, GridOverCursor, GridOverCursorInfo>,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            update_grid_cursor_info_on_changes::<C, GridSelectionCursor, GridSelectionCursorInfo>,
        );
        app.add_systems(PostUpdate, update_selection_cursor_info_ui);

        match self.generation_view_mode {
            GenerationViewMode::StepByStepTimed {
                steps_count,
                interval_ms,
            } => {
                app.add_systems(
                    Update,
                    (
                        insert_void_nodes_to_new_generations::<C, A, T>,
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
                        insert_void_nodes_to_new_generations::<C, A, T>,
                        step_by_step_input_update::<C>,
                        update_generation_view::<C, A, T>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (generate_all::<C>, update_generation_view::<C, A, T>).chain(),
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
    pub prev_node: KeyCode,
    pub next_node: KeyCode,
    pub cursor_x_axis: KeyCode,
    pub cursor_y_axis: KeyCode,
    pub cursor_z_axis: KeyCode,

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
            prev_node: KeyCode::Left,
            next_node: KeyCode::Right,
            cursor_x_axis: KeyCode::X,
            cursor_y_axis: KeyCode::Y,
            cursor_z_axis: KeyCode::Z,
            unpause: KeyCode::Space,
            step: KeyCode::Down,
            continuous_step: KeyCode::Up,
        }
    }
}

/// Component used to store model indexes of models with no assets, just to be able to skip their generation when stepping
#[derive(Component, bevy::prelude::Deref)]
pub struct VoidNodes(pub HashSet<ModelIndex>);

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

#[cfg(feature = "picking")]
pub fn insert_over_cursor_to_new_generations<C: CoordinateSystem>(
    mut commands: Commands,
    mut new_generations: Query<
        (Entity, &GridDefinition<C>, &Generator<C>),
        Without<GridOverCursor>,
    >,
) {
    for (gen_entity, _grid, _generation) in new_generations.iter_mut() {
        commands.entity(gen_entity).insert((
            ActiveGridCursor,
            GridOverCursor(GridCursor {
                color: Color::BLUE,
                node_index: 0,
                position: GridPosition::new(0, 0, 0),
                marker: None,
            }),
            GridOverCursorInfo(GridCursorInfo::new()),
        ));
    }
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
    mut observed_generations: Query<&mut Generator<C>, With<QueuedObserver>>,
) {
    for mut generation in observed_generations.iter_mut() {
        if generation_control.status == GenerationControlStatus::Ongoing {
            match generation.generate() {
                Ok(gen_info) => {
                    info!(
                        "Generation done, try_count: {}, seed: {}; grid: {}",
                        gen_info.try_count,
                        generation.seed(),
                        generation.grid()
                    );
                }
                Err(GeneratorError { node_index }) => {
                    warn!(
                        "Generation Failed at node {}, seed: {}; grid: {}",
                        node_index,
                        generation.seed(),
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
    mut observed_generations: Query<(&mut Generator<C>, &VoidNodes), With<QueuedObserver>>,
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
    mut observed_generations: Query<(&mut Generator<C>, &VoidNodes), With<QueuedObserver>>,
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
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut generators: Query<(
        Entity,
        &GridDefinition<C>,
        &AssetSpawner<A, T>,
        &mut QueuedObserver,
    )>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    for (gen_entity, grid, asset_spawner, mut observer) in generators.iter_mut() {
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
                    spawn_marker(&mut commands, grid, gen_entity, Color::RED, node_index);
                }
            }
        }

        if reinitialized {
            for existing_node in existing_nodes.iter() {
                commands.entity(existing_node).despawn_recursive();
            }
            marker_events.send(MarkerDespawnEvent::ClearAll);
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
                            generation.seed(),
                            generation.grid()
                        );
                        if generation_control.pause_when_done {
                            generation_control.status = GenerationControlStatus::Paused;
                        }
                        break;
                    }
                }
            }
            Err(GeneratorError { node_index }) => {
                warn!(
                    "Generation Failed at node {}, seed: {}; grid: {}",
                    node_index,
                    generation.seed(),
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

#[cfg(feature = "picking")]
#[derive(Component, Debug, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct GridOverCursor(pub GridCursor);

#[derive(Component, Debug, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct GridSelectionCursor(pub GridCursor);

#[derive(Debug)]
pub struct GridCursorInfo {
    models: Vec<ModelInfo>,
}
impl GridCursorInfo {
    fn new() -> Self {
        Self { models: Vec::new() }
    }
}

#[cfg(feature = "picking")]
#[derive(Component, Debug, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct GridOverCursorInfo(pub GridCursorInfo);

#[derive(Component, Debug, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub struct GridSelectionCursorInfo(pub GridCursorInfo);

#[cfg(feature = "picking")]
#[derive(Event, Deref, DerefMut)]
pub struct NodeOverEvent(pub Entity);

#[cfg(feature = "picking")]
impl From<ListenerInput<Pointer<Over>>> for NodeOverEvent {
    fn from(event: ListenerInput<Pointer<Over>>) -> Self {
        NodeOverEvent(event.listener())
    }
}

#[cfg(feature = "picking")]
#[derive(Event, Deref, DerefMut)]
pub struct NodeSelectedEvent(pub Entity);

#[cfg(feature = "picking")]
pub fn insert_grid_cursor_picking_handlers_to_spawned_nodes<C: CoordinateSystem>(
    mut commands: Commands,
    spawned_nodes: Query<Entity, Added<SpawnedNode>>,
) {
    use bevy_mod_picking::{pointer::PointerButton, prelude::ListenerMut};

    for node in spawned_nodes.iter() {
        commands
            .entity(node)
            .try_insert(On::<Pointer<Over>>::send_event::<NodeOverEvent>());
        commands.entity(node).try_insert(On::<Pointer<Down>>::run(
            move |event: ListenerMut<Pointer<Down>>,
                  mut selection_events: EventWriter<NodeSelectedEvent>| {
                if event.button == PointerButton::Primary {
                    selection_events.send(NodeSelectedEvent(event.listener()));
                }
            },
        ));
    }
}

#[cfg(feature = "picking")]
pub fn picking_update_grid_cursor_position<
    C: CoordinateSystem,
    W: Component + DerefMut<Target = GridCursor>,
    E: Event + DerefMut<Target = Entity>,
>(
    mut events: EventReader<E>,
    mut commands: Commands,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<(&SpawnedNode, &Parent)>,
    mut parent: Query<(&mut W, &GridDefinition<C>)>,
) {
    for event in events.read().last() {
        if let Ok((node, node_parent)) = nodes.get_mut(**event) {
            let parent_entity = node_parent.get();
            if let Ok((mut cursor, grid)) = parent.get_mut(parent_entity) {
                if cursor.node_index != node.0 {
                    cursor.node_index = node.0;
                    cursor.position = grid.pos_from_index(node.0);

                    if let Some(previous_cursor_entity) = cursor.marker {
                        marker_events.send(MarkerDespawnEvent::Remove {
                            marker_entity: previous_cursor_entity,
                        });
                    }
                    let marker_entity = commands
                        .spawn(GridMarker::new(cursor.color, cursor.position.clone()))
                        .id();
                    commands.entity(parent_entity).add_child(marker_entity);
                    cursor.marker = Some(marker_entity);
                }
            }
        }
    }
}

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
