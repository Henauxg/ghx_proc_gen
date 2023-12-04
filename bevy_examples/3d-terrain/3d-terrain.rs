use std::time::Duration;

use bevy::{
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    utils::HashMap,
};

use bevy_ghx_utilities::camera::{pan_orbit_camera, PanOrbitCamera};
use ghx_proc_gen::{
    generator::{
        builder::GeneratorBuilder, node::GeneratedNode, observer::QueuedObserver,
        rules::RulesBuilder, GenerationStatus, Generator, ModelSelectionHeuristic,
        NodeSelectionHeuristic, RngMode,
    },
    grid::{direction::Cartesian3D, GridDefinition},
};

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

#[derive(Resource)]
struct Generation {
    models_assets: HashMap<usize, Handle<Scene>>,
    gen: Generator<Cartesian3D>,
    observer: QueuedObserver,
}

#[derive(Resource)]
struct GenerationTimer(Timer);

const SCALE_FACTOR: f32 = 1. / 40.; // Models are 40 voxels wide
const MODEL_SCALE: Vec3 = Vec3::new(SCALE_FACTOR, SCALE_FACTOR, SCALE_FACTOR);

fn setup_scene(mut commands: Commands) {
    // Camera
    let camera_position = Vec3::new(-2.5, 4.5, 9.0);
    let radius = camera_position.length();
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PanOrbitCamera {
            radius,
            ..Default::default()
        },
    ));
    // Scene lights
    commands.insert_resource(AmbientLight {
        color: Color::SEA_GREEN,
        brightness: 0.1,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.,
            color: Color::SEA_GREEN,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 1.6,
            ..default()
        }
        .into(),
        ..default()
    });
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load rules
    let (models_asset_paths, models, sockets_connections) = rules_and_assets();

    // Create generator
    let rules = RulesBuilder::new_cartesian_3d(models, sockets_connections)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_3d(35, 5, 35, false);
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_max_retry_count(250)
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
            models_assets.insert(
                index,
                asset_server.load(format!("3d_terrain/{path}.glb#Scene0")),
            );
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
}

#[derive(Component)]
struct SpawnedNode;

fn spawn_node(
    commands: &mut Commands,
    models_assets: &HashMap<usize, Handle<Scene>>,
    grid: &GridDefinition<Cartesian3D>,
    node: &GeneratedNode,
    node_index: usize,
) {
    if let Some(asset) = models_assets.get(&node.model_index) {
        let x_offset = grid.size_x() as f32 / 2.;
        let z_offset = grid.size_z() as f32 / 2.;
        let pos = grid.get_position(node_index);
        commands.spawn((
            SceneBundle {
                scene: asset.clone(),
                transform: Transform::from_xyz(
                    (pos.x as f32) - x_offset,
                    pos.y as f32,
                    z_offset - (pos.z as f32),
                )
                .with_scale(MODEL_SCALE)
                .with_rotation(Quat::from_rotation_y(f32::to_radians(
                    node.rotation.value() as f32,
                ))),
                ..default()
            },
            SpawnedNode,
        ));
    }
}

#[derive(Event)]
struct GenerationFailedEvent;

fn select_and_propagate(
    commands: &mut Commands,
    generation_failed_events: &mut EventWriter<GenerationFailedEvent>,
    generation: &mut ResMut<Generation>,
) {
    match generation.gen.select_and_propagate() {
        Ok(status) => {
            let updates = generation.observer.update();
            info!("Spawning {} node(s)", updates.len());
            for update in updates {
                spawn_node(
                    commands,
                    &generation.models_assets,
                    generation.gen.grid(),
                    &update.node(),
                    update.node_index(),
                );
            }
            match status {
                GenerationStatus::Ongoing => (),
                GenerationStatus::Done => info!("Generation done"),
            }
        }
        Err(_) => {
            generation_failed_events.send(GenerationFailedEvent);
        }
    }
}

fn clear_nodes(
    mut commands: Commands,
    mut generation_failed_events: EventReader<GenerationFailedEvent>,
    nodes: Query<(Entity, &SpawnedNode)>,
) {
    for _event in generation_failed_events.read() {
        info!("Generation failed");
        for (entity, _node) in nodes.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn step_by_step_input_update(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    mut generation_failed_events: EventWriter<GenerationFailedEvent>,
    mut generation: ResMut<Generation>,
) {
    if keys.just_pressed(KeyCode::Space) {
        select_and_propagate(
            &mut commands,
            &mut generation_failed_events,
            &mut generation,
        );
    }
}

fn step_by_step_timed_update(
    mut commands: Commands,
    mut generation: ResMut<Generation>,
    mut generation_failed_events: EventWriter<GenerationFailedEvent>,
    mut timer: ResMut<GenerationTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        select_and_propagate(
            &mut commands,
            &mut generation_failed_events,
            &mut generation,
        );
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins(DefaultPlugins);
    app.add_event::<GenerationFailedEvent>();
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(Update, pan_orbit_camera)
        .add_systems(PostUpdate, clear_nodes);

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
