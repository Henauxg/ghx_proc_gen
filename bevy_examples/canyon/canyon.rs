use std::f32::consts::PI;

use bevy::{log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*};

use bevy_examples::{
    anim::{ease_in_cubic, SpawningScaleAnimation},
    camera::{pan_orbit_camera, PanOrbitCamera},
    plugin::{scene_node_spawner, ProcGenExamplesPlugin},
    utils::load_assets,
    Generation, GenerationControl, GenerationViewMode,
};
use bevy_ghx_proc_gen::{
    grid::{
        view::{DebugGridView, DebugGridView3d, DebugGridViewConfig3d},
        Grid,
    },
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, node_heuristic::NodeSelectionHeuristic, rules::RulesBuilder,
            ModelSelectionHeuristic, RngMode,
        },
        grid::{direction::Cartesian3D, GridDefinition},
    },
};

use crate::rules::rules_and_assets;

mod rules;

// --------------------------------------------
/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

/// Change to visualize void nodes with a transparent asset
const SEE_VOID_NODES: bool = false;

const AUTO_ORBIT_CAMERA: bool = true;

/// Change this to change the map size.
const GRID_HEIGHT: u32 = 5;
const GRID_X: u32 = 40;
const GRID_Z: u32 = 40;
// --------------------------------------------

const ASSETS_PATH: &str = "canyon";
/// Size of a block in world units
const NODE_SIZE: f32 = 1.;
const NODE_SCALE: Vec3 = Vec3::new(NODE_SIZE, NODE_SIZE, NODE_SIZE);

const ASSETS_SCALE_FACTOR: f32 = NODE_SIZE / 2.;
const ASSETS_SCALE: Vec3 = Vec3::new(
    ASSETS_SCALE_FACTOR,
    ASSETS_SCALE_FACTOR,
    ASSETS_SCALE_FACTOR,
);

fn setup_scene(
    mut commands: Commands,
    mut _meshes: ResMut<Assets<Mesh>>,
    mut _materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    let camera_position = Vec3::new(0., 3. * GRID_HEIGHT as f32, 1.7 * GRID_Z as f32 / 2.);
    let radius = camera_position.length();
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PanOrbitCamera {
            radius,
            auto_orbit: AUTO_ORBIT_CAMERA,
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
    let gen = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_max_retry_count(250)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumEntropy)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();

    // Load assets
    let models_assets = load_assets(&asset_server, assets_definitions, ASSETS_PATH, "glb#Scene0");

    let grid_entity = commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_translation(Vec3 {
                x: -(grid.size_x() as f32) / 2.,
                y: 0.,
                z: -(grid.size_z() as f32) / 2.,
            })),
            Grid { def: grid },
            DebugGridView3d {
                config: DebugGridViewConfig3d {
                    node_size: NODE_SCALE,
                },
                view: DebugGridView::new(false, true, Color::GRAY),
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
        ProcGenExamplesPlugin::<Cartesian3D, Scene, SceneBundle>::new(GENERATION_VIEW_MODE),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(Update, pan_orbit_camera);

    app.run();
}
