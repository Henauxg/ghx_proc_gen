use std::collections::HashMap;

use bevy::{
    log::LogPlugin,
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};

use bevy_examples::{
    anim::{ease_in_cubic, SpawningScaleAnimation},
    camera::{pan_orbit_camera, PanOrbitCamera},
    plugin::{scene_node_spawner, ProcGenExamplesPlugin},
    utils::toggle_debug_grid_visibility,
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
    let camera_position = Vec3::new(-2.5, 1.5, 9.0);
    let _radius = camera_position.length();
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PanOrbitCamera {
            focus: camera_position,
            radius: 0.01,
            ..Default::default()
        },
        FogSettings {
            color: Color::rgba(0.2, 0.15, 0.1, 1.0),
            falloff: FogFalloff::Linear {
                start: 12.0,
                end: 30.0,
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
        transform: Transform::from_scale(Vec3::splat(100.0)),
        ..default()
    });
    // Ceiling
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::default())),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("888888").unwrap(),
            // unlit: true,
            // cull_mode: None,
            ..default()
        }),
        transform: Transform::from_scale(Vec3::splat(100.0))
            .with_translation(Vec3::new(0., GRID_HEIGHT as f32, 0.))
            .with_rotation(Quat::from_rotation_x(f32::to_radians(180.))),
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
            illuminance: 10000.,
            color: Color::WHITE,
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
    let grid = GridDefinition::new_cartesian_3d(GRID_X, GRID_HEIGHT, GRID_Z, false);
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
    let mut models_assets = HashMap::new();
    for (index, path) in models_asset_paths.iter().enumerate() {
        if let Some(path) = path {
            models_assets.insert(
                index,
                asset_server.load(format!("pillars/{path}.glb#Scene0")),
            );
        }
    }

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
