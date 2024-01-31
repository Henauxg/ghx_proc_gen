use std::f32::consts::PI;

use bevy::{log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*};

use bevy_examples::{
    anim::SpawningScaleAnimation,
    camera::{pan_orbit_camera, PanOrbitCamera},
    plugin::ProcGenExamplesPlugin,
    utils::load_assets,
};
use bevy_ghx_proc_gen::{
    gen::{
        assets::AssetSpawner,
        debug_plugin::{GenerationControl, GenerationViewMode},
    },
    grid::{
        view::{DebugGridView, DebugGridViewConfig3d},
        DebugGridView3dBundle,
    },
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, node_heuristic::NodeSelectionHeuristic, rules::RulesBuilder,
            ModelSelectionHeuristic, RngMode,
        },
        grid::{direction::Cartesian3D, GridDefinition},
    },
    GeneratorBundle,
};
use rand::Rng;
use rules::{CustomComponents, RotationRandomizer, ScaleRandomizer, WindRotation};

use crate::rules::rules_and_assets;

mod rules;

// --------------------------------------------
/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::StepByStepTimed {
    steps_count: 10,
    interval_ms: 5,
};

/// Change to visualize void nodes with a transparent asset
const SEE_VOID_NODES: bool = false;

/// Change this to change the map size.
const GRID_HEIGHT: u32 = 5;
const GRID_X: u32 = 40;
const GRID_Z: u32 = 40;
// --------------------------------------------

const ASSETS_PATH: &str = "canyon";
/// Size of a block in world units
const BLOCK_SIZE: f32 = 1.;
/// Size of a grid node in world units
const NODE_SIZE: Vec3 = Vec3::new(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE);

const ASSETS_SCALE_FACTOR: f32 = BLOCK_SIZE / 2.;
const ASSETS_SCALE: Vec3 = Vec3::new(
    ASSETS_SCALE_FACTOR,
    ASSETS_SCALE_FACTOR,
    ASSETS_SCALE_FACTOR,
);

fn setup_scene(mut commands: Commands) {
    // Camera
    let camera_position = Vec3::new(0., 1.5 * GRID_HEIGHT as f32, 1.5 * GRID_Z as f32 / 2.);
    let look_target = Vec3::new(0., -10., 0.);
    let radius = (look_target - camera_position).length();
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(camera_position)
                .looking_at(look_target, Vec3::Y),
            ..default()
        },
        PanOrbitCamera {
            radius,
            ..Default::default()
        },
    ));

    // Scene lights
    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.05,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 8000.,
            color: Color::rgb(1.0, 0.85, 0.65),
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(5.0, 10.0, 2.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0., -PI / 5., -PI / 3.),
            ..default()
        },
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            illuminance: 4000.,
            color: Color::ORANGE_RED,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(5.0, 10.0, 2.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0., PI * 4. / 5., -PI / 3.),
            ..default()
        },
        ..default()
    });
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    let (assets_definitions, models, socket_collection) = rules_and_assets();

    // Create generator
    let rules = RulesBuilder::new_cartesian_3d(models, socket_collection)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_3d(GRID_X, GRID_HEIGHT, GRID_Z, false, false, false);

    let mut gen_builder = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_max_retry_count(50)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumEntropy)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability);
    let observer = gen_builder.add_queued_observer();
    let generator = gen_builder.build().unwrap();

    // Load assets
    let models_assets = load_assets::<Scene, CustomComponents>(
        &asset_server,
        assets_definitions,
        ASSETS_PATH,
        "glb#Scene0",
    );

    commands.spawn((
        GeneratorBundle {
            spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3 {
                x: -(grid.size_x() as f32) / 2.,
                y: 0.,
                z: -(grid.size_z() as f32) / 2.,
            })),
            grid,
            generator,
            asset_spawner: AssetSpawner::new(
                models_assets,
                NODE_SIZE,
                // We spawn assets with a scale of 0 since we animate their scale in the examples
                Vec3::ZERO,
            ),
        },
        observer,
        DebugGridView3dBundle {
            config: DebugGridViewConfig3d {
                node_size: NODE_SIZE,
            },
            view: DebugGridView::new(false, true, Color::GRAY),
        },
    ));

    commands.insert_resource(GenerationControl::new(true, true, false));
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins((
        DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,ghx_proc_gen=debug".into(),
            level: bevy::log::Level::DEBUG,
        }),
        ProcGenExamplesPlugin::<Cartesian3D, Handle<Scene>, CustomComponents>::new(
            GENERATION_VIEW_MODE,
            ASSETS_SCALE,
        ),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(
            Update,
            (
                pan_orbit_camera,
                apply_wind,
                randomize_spawn_scale,
                randomize_spawn_rotation,
            ),
        );

    app.run();
}

pub fn apply_wind(
    time: Res<Time>,
    mut altered_transforms: Query<&mut Transform, With<WindRotation>>,
) {
    for mut transform in altered_transforms.iter_mut() {
        transform.rotation = Quat::from_rotation_z(2. * time.elapsed_seconds_wrapped());
    }
}

pub fn randomize_spawn_scale(
    mut commands: Commands,
    mut altered_transforms: Query<(Entity, &mut SpawningScaleAnimation), With<ScaleRandomizer>>,
) {
    let mut rng = rand::thread_rng();
    for (entity, mut spawning_scale_animation) in altered_transforms.iter_mut() {
        spawning_scale_animation.final_scale =
            spawning_scale_animation.final_scale * rng.gen_range(0.7..1.3);
        commands.entity(entity).remove::<ScaleRandomizer>();
    }
}

pub fn randomize_spawn_rotation(
    mut commands: Commands,
    mut altered_transforms: Query<(Entity, &mut Transform), With<RotationRandomizer>>,
) {
    let mut rng = rand::thread_rng();
    for (entity, mut transform) in altered_transforms.iter_mut() {
        transform.rotate_y(rng.gen_range(-45.0..45.0));
        commands.entity(entity).remove::<RotationRandomizer>();
    }
}
