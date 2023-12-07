use std::time::Duration;

use bevy::{log::LogPlugin, prelude::*, utils::HashMap};

use bevy_ghx_proc_gen::{
    grid::{spawn_debug_grids, DebugGridView, DebugGridViewConfig, Grid},
    lines::LineMaterial,
    proc_gen::{
        generator::{
            builder::GeneratorBuilder,
            node::GeneratedNode,
            observer::{GenerationUpdate, QueuedObserver},
            rules::RulesBuilder,
            GenerationStatus, Generator, ModelSelectionHeuristic, NodeSelectionHeuristic, RngMode,
        },
        grid::{direction::Cartesian2D, GridDefinition},
    },
};
use bevy_ghx_utilities::camera::{pan_orbit_camera, PanOrbitCamera};

use crate::rules::rules_and_assets;

mod rules;

#[derive(PartialEq, Eq)]
pub enum GenerationViewMode {
    StepByStep(u64),
    StepByStepPaused,
    Final,
}

/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

const GRID_X: u32 = 20;
const GRID_Y: u32 = 20;

#[derive(Resource)]
struct Generation {
    models_assets: HashMap<usize, Handle<Image>>,
    gen: Generator<Cartesian2D>,
    observer: QueuedObserver,
}

#[derive(Resource)]
struct GenerationTimer(Timer);

/// Size of a block in world units
const TILE_SIZE: f32 = 32.;

fn setup_scene(mut commands: Commands) {
    // Camera
    commands.spawn((Camera2dBundle::default(), PanOrbitCamera::default()));
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load rules
    let (models_asset_paths, models, sockets_connections) = rules_and_assets();

    // Create generator
    let rules = RulesBuilder::new_cartesian_2d(models, sockets_connections)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_2d(GRID_X, GRID_Y, false);
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();
    let observer = QueuedObserver::new(&mut generator);
    info!("Seed: {}", generator.get_seed());

    // Load assets
    let mut models_assets = HashMap::new();
    for (index, path) in models_asset_paths.iter().enumerate() {
        if let Some(path) = path {
            models_assets.insert(index, asset_server.load(format!("tiles/{path}.png")));
        }
    }

    match GENERATION_VIEW_MODE {
        GenerationViewMode::StepByStepPaused => (),
        GenerationViewMode::StepByStep(interval) => commands.insert_resource(GenerationTimer(
            Timer::new(Duration::from_millis(interval), TimerMode::Repeating),
        )),
        GenerationViewMode::Final => {
            let output = generator.generate().unwrap();
            for (node_index, node) in output.nodes().iter().enumerate() {
                spawn_node(
                    &mut commands,
                    &models_assets,
                    generator.grid(),
                    node,
                    node_index,
                );
            }
        }
    }

    commands.insert_resource(Generation {
        models_assets,
        gen: generator,
        observer,
    });

    commands.spawn((
        SpatialBundle::from_transform(Transform::from_translation(Vec3 {
            x: 0., // -(grid.size_x() as f32) / 2.
            y: 0.,
            z: 0.,
        })),
        Grid { def: grid },
        DebugGridViewConfig {
            node_size: Vec3::splat(TILE_SIZE),
            color: Color::GRAY.with_a(0.),
        },
    ));
}

#[derive(Component)]
struct SpawnedNode;

fn spawn_node(
    commands: &mut Commands,
    models_assets: &HashMap<usize, Handle<Image>>,
    grid: &GridDefinition<Cartesian2D>,
    node: &GeneratedNode,
    node_index: usize,
) {
    info!("Spawning {:?} at node index {}", node, node_index);
    if let Some(asset) = models_assets.get(&node.model_index) {
        let x_offset = TILE_SIZE * grid.size_x() as f32 / 2.;
        let y_offset = TILE_SIZE * grid.size_y() as f32 / 2.;
        let pos = grid.get_position(node_index);
        commands.spawn((
            SpriteBundle {
                texture: asset.clone(),
                transform: Transform::from_xyz(
                    TILE_SIZE * pos.x as f32 - x_offset,
                    TILE_SIZE * pos.y as f32 - y_offset,
                    0.,
                )
                .with_rotation(Quat::from_rotation_z(f32::to_radians(
                    node.rotation.value() as f32,
                ))),
                ..default()
            },
            SpawnedNode,
        ));
    }
}

fn select_and_propagate(
    commands: &mut Commands,
    generation: &mut ResMut<Generation>,
    nodes: Query<Entity, With<SpawnedNode>>,
) {
    match generation.gen.select_and_propagate() {
        Ok(status) => match status {
            GenerationStatus::Ongoing => (),
            GenerationStatus::Done => info!("Generation done"),
        },
        Err(_) => {
            info!("Generation Failed")
        }
    }
    // Process the observer queue even if generation failed
    let updates = generation.observer.dequeue_all();
    // Buffer the nodes to spawn in case a generation failure invalidate some.
    let mut nodes_to_spawn = vec![];
    let mut despawn_nodes = false;
    for update in updates {
        match update {
            GenerationUpdate::Generated {
                node_index,
                generated_node,
            } => {
                nodes_to_spawn.push((node_index, generated_node));
            }
            GenerationUpdate::Reinitialized | GenerationUpdate::Failed => {
                nodes_to_spawn.clear();
                despawn_nodes = true;
            }
        }
    }
    if despawn_nodes {
        for entity in nodes.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }

    for (node_index, generated_node) in nodes_to_spawn {
        spawn_node(
            commands,
            &generation.models_assets,
            generation.gen.grid(),
            &generated_node,
            node_index,
        );
    }
}

fn step_by_step_input_update(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    mut generation: ResMut<Generation>,
    nodes: Query<Entity, With<SpawnedNode>>,
) {
    if keys.pressed(KeyCode::Space) || buttons.just_pressed(MouseButton::Left) {
        select_and_propagate(&mut commands, &mut generation, nodes);
    }
}

fn step_by_step_timed_update(
    mut commands: Commands,
    mut generation: ResMut<Generation>,
    mut timer: ResMut<GenerationTimer>,
    time: Res<Time>,
    nodes: Query<Entity, With<SpawnedNode>>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        select_and_propagate(&mut commands, &mut generation, nodes);
    }
}

fn toggle_debug_grid_visibility(
    keys: Res<Input<KeyCode>>,
    mut debug_grids: Query<&mut Visibility, With<DebugGridView>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        for mut view_visibility in debug_grids.iter_mut() {
            *view_visibility = match *view_visibility {
                Visibility::Inherited => Visibility::Hidden,
                Visibility::Hidden => Visibility::Visible,
                Visibility::Visible => Visibility::Hidden,
            }
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(LogPlugin {
                filter: "info,wgpu_core=warn,wgpu_hal=warn,ghx_proc_gen=debug".into(),
                level: bevy::log::Level::DEBUG,
            })
            .set(ImagePlugin::default_nearest()),
        MaterialPlugin::<LineMaterial>::default(),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(Update, pan_orbit_camera)
        .add_systems(Update, spawn_debug_grids::<Cartesian2D>)
        .add_systems(Update, toggle_debug_grid_visibility);

    match GENERATION_VIEW_MODE {
        GenerationViewMode::StepByStep(_) => {
            app.add_systems(Update, step_by_step_timed_update);
        }
        GenerationViewMode::StepByStepPaused => {
            app.add_systems(Update, step_by_step_input_update);
        }
        GenerationViewMode::Final => (),
    };

    app.run();
}
