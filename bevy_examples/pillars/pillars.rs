use std::f32::consts::PI;

use bevy::{log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*};

use bevy_examples::{
    anim::{ease_in_cubic, SpawningScaleAnimation},
    camera::{pan_orbit_camera, PanOrbitCamera},
    plugin::{scene_node_spawner, ProcGenExamplesPlugin},
    utils::{load_assets, toggle_debug_grid_visibility},
    Generation, GenerationControl, GenerationViewMode,
};
use bevy_ghx_proc_gen::{
    grid::{spawn_debug_grids, DebugGridViewConfig, Grid},
    lines::LineMaterial,
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, rules::RulesBuilder, ModelSelectionHeuristic,
            NodeSelectionHeuristic, RngMode,
        },
        grid::{direction::Cartesian3D, GridDefinition},
    },
};

use crate::rules::rules_and_assets;

mod rules;

// --------------------------------------------
/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::StepByStep(15);

/// Change this to change the map size.
const GRID_HEIGHT: u32 = 7;
const GRID_X: u32 = 80;
const GRID_Z: u32 = 80;
// --------------------------------------------

/// Size of a block in world units
const NODE_SIZE: f32 = 1.;
const NODE_SCALE: Vec3 = Vec3::new(NODE_SIZE, NODE_SIZE, NODE_SIZE);

const ASSETS_SCALE_FACTOR: f32 = NODE_SIZE / 4.; // Models are 4 units wide
const ASSETS_SCALE: Vec3 = Vec3::new(
    ASSETS_SCALE_FACTOR,
    ASSETS_SCALE_FACTOR,
    ASSETS_SCALE_FACTOR,
);

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    let camera_position = Vec3::new(0., 1.5 * GRID_HEIGHT as f32, GRID_Z as f32 / 3.);
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
        FogSettings {
            color: Color::rgba(0.2, 0.15, 0.1, 1.0),
            falloff: FogFalloff::Linear {
                start: 20.0,
                end: 45.0,
            },
            ..default()
        },
    ));
    // Sky
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::default())),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("888888").unwrap(),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
        ..default()
    });
    // Ground
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::default())),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("888888").unwrap(),
            // unlit: true,
            // cull_mode: None,
            ..default()
        }),
        transform: Transform::from_scale(Vec3::splat(100.0)).with_translation(Vec3::new(
            0.,
            NODE_SIZE / 2.,
            0.,
        )),
        ..default()
    });

    // Scene lights
    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.02,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 8000.,
            color: Color::ORANGE_RED,
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
    // Load rules
    let (models_asset_paths, models, socket_collection) = rules_and_assets();

    // Create generator
    let rules = RulesBuilder::new_cartesian_3d(models, socket_collection)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_3d(GRID_X, GRID_HEIGHT, GRID_Z, false, false, false);
    let gen = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_max_retry_count(250)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();
    info!("Seed: {}", gen.get_seed());

    // Load assets
    let models_assets = load_assets(&asset_server, models_asset_paths, "pillars", "glb#Scene0");

    let grid_entity = commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_translation(Vec3 {
                x: -(grid.size_x() as f32) / 2.,
                y: 0.,
                z: -(grid.size_z() as f32) / 2.,
            })),
            Grid { def: grid },
            DebugGridViewConfig {
                node_size: NODE_SCALE,
                color: Color::GRAY.with_a(0.),
            },
        ))
        .id();

    commands.insert_resource(Generation::new(
        models_assets,
        gen,
        NODE_SCALE,
        grid_entity,
        Vec3::ZERO,
        scene_node_spawner,
        Some(SpawningScaleAnimation::new(
            0.8,
            ASSETS_SCALE,
            ease_in_cubic,
        )),
        false,
    ));

    commands.insert_resource(GenerationControl::new(true, true, true));
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins((
        DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,ghx_proc_gen=debug".into(),
            level: bevy::log::Level::DEBUG,
        }),
        MaterialPlugin::<LineMaterial>::default(),
        ProcGenExamplesPlugin::<Cartesian3D, Scene, SceneBundle>::new(GENERATION_VIEW_MODE),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(Update, pan_orbit_camera)
        .add_systems(Update, spawn_debug_grids::<Cartesian3D>)
        .add_systems(Update, toggle_debug_grid_visibility);

    app.run();
}
