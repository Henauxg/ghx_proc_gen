use std::time::Duration;

use bevy::{
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    utils::HashMap,
};

use ghx_bevy_utilities::{pan_orbit_camera, PanOrbitCamera};
use ghx_proc_gen::{
    generator::{
        builder::GeneratorBuilder, observer::QueuedStatefulObserver, rules::Rules,
        GenerationStatus, Generator, ModelSelectionHeuristic, NodeSelectionHeuristic, RngMode,
    },
    grid::{direction::Cartesian3D, GridDefinition},
};

use crate::rules::rules_and_assets;

mod rules;

#[derive(PartialEq, Eq)]
enum GenerationViewMode {
    StepByStep(u64),
    StepByStepPaused,
    Final,
}

#[derive(Resource)]
struct Generation {
    generator: Generator<Cartesian3D>,
    observer: QueuedStatefulObserver<Cartesian3D>,
    // timer: Timer,
}

#[derive(Resource)]
struct GenerationTimer(Timer);

/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

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
            illuminance: 1000.,
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
    let (models_asset_paths, models, sockets_connections) = rules_and_assets();

    let rules = Rules::new_cartesian_3d(models, sockets_connections).unwrap();
    let grid = GridDefinition::new_cartesian_3d(35, 35, 5, false);
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_max_retry_count(250)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();
    let mut observer = QueuedStatefulObserver::new(&mut generator);
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
            generator.generate_without_output().unwrap();
            observer.update();
            let data_grid = observer.grid_data();
            let x_offset = data_grid.grid().size_x() as f32 / 2.;
            let z_offset = data_grid.grid().size_y() as f32 / 2.;
            for z in (0..data_grid.grid().size_y()).rev() {
                for x in 0..data_grid.grid().size_x() {
                    for y in 0..data_grid.grid().size_z() {
                        match data_grid.get_3d(x, z, y) {
                            None => (),
                            Some(node) => {
                                if let Some(asset) = models_assets.get(&node.index) {
                                    commands.spawn(SceneBundle {
                                        scene: asset.clone(),
                                        // Y is up in Bevy.
                                        transform: Transform::from_xyz(
                                            (x as f32) - x_offset,
                                            y as f32,
                                            z_offset - (z as f32),
                                        )
                                        .with_scale(MODEL_SCALE)
                                        .with_rotation(Quat::from_rotation_y(f32::to_radians(
                                            node.rotation.value() as f32,
                                        ))),
                                        ..default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    commands.insert_resource(Generation {
        generator,
        observer,
    });
}

fn select_and_propagate(generation: &mut ResMut<Generation>) {
    match generation.generator.select_and_propagate() {
        Ok(status) => match status {
            GenerationStatus::Ongoing => {
                generation.observer.update();
                // TODO Stateless observer
            }
            GenerationStatus::Done => {
                // TODO
            }
        },
        Err(_) => info!("Generation failed"),
    }
}

fn step_by_step_input_update(keys: Res<Input<KeyCode>>, mut generation: ResMut<Generation>) {
    let should_iterate = keys.just_pressed(KeyCode::NumpadEnter);

    if should_iterate {
        select_and_propagate(&mut generation);
    }
}

fn step_by_step_timed_update(
    mut generation: ResMut<Generation>,
    mut timer: ResMut<GenerationTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());
    let should_iterate = timer.0.finished();

    if should_iterate {
        select_and_propagate(&mut generation);
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup_generator)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, pan_orbit_camera);

    match GENERATION_VIEW_MODE {
        GenerationViewMode::StepByStep(_) => {
            app.add_systems(Startup, step_by_step_timed_update);
        }
        GenerationViewMode::StepByStepPaused => {
            app.add_systems(Startup, step_by_step_input_update);
        }
        GenerationViewMode::Final => (),
    };

    app.run();
}
