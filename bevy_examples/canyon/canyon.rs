use std::f32::consts::PI;

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::{GRAY, ORANGE_RED},
    light::DirectionalLightShadowMap,
    log::LogPlugin,
    prelude::*,
};

use bevy_examples::{
    anim::SpawningScaleAnimation, plugin::ProcGenExamplesPlugin, utils::load_assets,
};
use bevy_ghx_proc_gen::{
    bevy_ghx_grid::debug_plugin::{view::DebugGridView, DebugGridView3dBundle},
    debug_plugin::generation::{GenerationControl, GenerationViewMode},
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, node_heuristic::NodeSelectionHeuristic, rules::RulesBuilder,
            ModelSelectionHeuristic, RngMode,
        },
        ghx_grid::cartesian::{coordinates::Cartesian3D, grid::CartesianGrid},
    },
    spawner_plugin::NodesSpawner,
};

use rand::Rng;
use rules::{RotationRandomizer, ScaleRandomizer, WindRotation};

use crate::rules::rules_and_assets;

mod rules;

// -----------------  Configurable values ---------------------------
/// Modify this value to control the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

/// Modify to visualize void nodes with a transparent asset
const SEE_VOID_NODES: bool = false;

/// Modify these values to control the map size.
const GRID_HEIGHT: u32 = 6;
const GRID_X: u32 = 30;
const GRID_Z: u32 = 30;
// ------------------------------------------------------------------

const ASSETS_PATH: &str = "canyon";
/// Size of a block in world units
const BLOCK_SIZE: f32 = 1.;
/// Size of a grid node in world units
const NODE_SIZE: Vec3 = Vec3::splat(BLOCK_SIZE);

const ASSETS_SCALE_FACTOR: f32 = BLOCK_SIZE / 2.;
const ASSETS_SCALE: Vec3 = Vec3::splat(ASSETS_SCALE_FACTOR);

fn setup_scene(mut commands: Commands) {
    // Camera
    let camera_position = Vec3::new(0., 2.5 * GRID_HEIGHT as f32, 1.8 * GRID_Z as f32 / 2.);
    let look_target = Vec3::new(0., 0., 0.);
    commands.spawn((
        Name::new("Camera"),
        Transform::from_translation(camera_position).looking_at(look_target, Vec3::Y),
        Camera3d::default(),
        FreeCamera {
            walk_speed: 30.0,
            run_speed: 50.0,
            scroll_factor: 0.0,
            ..default()
        },
    ));

    // Scene lights
    commands.insert_resource(GlobalAmbientLight {
        color: Color::Srgba(ORANGE_RED),
        brightness: 0.05,
        ..default()
    });
    commands.spawn((
        Name::new("Main light"),
        Transform {
            translation: Vec3::new(5.0, 10.0, 2.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0., -PI / 5., -PI / 3.),
            ..default()
        },
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 4000.,
            color: Color::srgb(1.0, 0.85, 0.65),
            ..default()
        },
    ));
    commands.spawn((
        Name::new("Back light"),
        Transform {
            translation: Vec3::new(5.0, 10.0, 2.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0., PI * 4. / 5., -PI / 3.),
            ..default()
        },
        DirectionalLight {
            shadows_enabled: false,
            illuminance: 2000.,
            color: Color::Srgba(ORANGE_RED),
            ..default()
        },
    ));
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    let (
        void_instance,
        sand_instance,
        water_instance,
        bridge_instance,
        assets_definitions,
        models,
        socket_collection,
    ) = rules_and_assets();

    // Create generator
    let rules = RulesBuilder::new_cartesian_3d(models, socket_collection)
        .build()
        .unwrap();
    let grid = CartesianGrid::new_cartesian_3d(GRID_X, GRID_HEIGHT, GRID_Z, false, false, false);

    let mut initial_constraints = grid.new_grid_data(None);
    // Force void nodes on the upmost layer
    let void_ref = Some(void_instance);
    initial_constraints.set_all_y(GRID_HEIGHT - 1, void_ref);
    // Force void nodes on the grid's "borders"
    initial_constraints.set_all_x(0, void_ref);
    initial_constraints.set_all_x(GRID_X - 1, void_ref);
    initial_constraints.set_all_z(0, void_ref);
    initial_constraints.set_all_z(GRID_Z - 1, void_ref);
    // Force sand nodes on the grid's "borders" ground
    let sand_ref = Some(sand_instance);
    initial_constraints.set_all_xy(0, 0, sand_ref);
    initial_constraints.set_all_xy(GRID_X - 1, 0, sand_ref);
    initial_constraints.set_all_yz(0, 0, sand_ref);
    initial_constraints.set_all_yz(0, GRID_Z - 1, sand_ref);
    // Let's force a small lake at the center
    let water_ref = Some(water_instance);
    for x in 2 * GRID_X / 5..3 * GRID_X / 5 {
        for z in 2 * GRID_Z / 5..3 * GRID_Z / 5 {
            *initial_constraints.get_3d_mut(x, 0, z) = water_ref;
        }
    }
    // We could hope for a water bridge, or force one !
    *initial_constraints.get_3d_mut(GRID_X / 2, GRID_HEIGHT / 2, GRID_Z / 2) =
        Some(bridge_instance);

    let mut gen_builder = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_max_retry_count(50)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumEntropy)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        // There are other methods to initialize the generation. See with_initial_nodes
        .with_initial_grid(initial_constraints)
        .unwrap();
    let observer = gen_builder.add_queued_observer();
    let generator = gen_builder.build().unwrap();

    // Load assets
    let models_assets =
        load_assets::<Scene>(&asset_server, assets_definitions, ASSETS_PATH, "glb#Scene0");

    commands.spawn((
        Transform::from_translation(Vec3 {
            x: -(grid.size_x() as f32) / 2.,
            y: 0.,
            z: -(grid.size_z() as f32) / 2.,
        }),
        grid,
        generator,
        NodesSpawner::new(
            models_assets,
            NODE_SIZE,
            // We spawn assets with a scale of 0 since we animate their scale in the examples
            Vec3::ZERO,
        ),
        observer,
        DebugGridView3dBundle {
            view: DebugGridView::new(false, true, Color::Srgba(GRAY), NODE_SIZE),
            ..default()
        },
    ));

    commands.insert_resource(GenerationControl {
        pause_on_error: false,
        ..Default::default()
    });
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins((
        DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=error,wgpu_hal=error,ghx_proc_gen=debug".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }),
        FreeCameraPlugin,
        ProcGenExamplesPlugin::<Cartesian3D, Handle<Scene>>::new(
            GENERATION_VIEW_MODE,
            ASSETS_SCALE,
        ),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(
            Update,
            (apply_wind, randomize_spawn_scale, randomize_spawn_rotation),
        );

    app.run();
}

pub fn apply_wind(
    time: Res<Time>,
    mut altered_transforms: Query<&mut Transform, With<WindRotation>>,
) {
    for mut transform in altered_transforms.iter_mut() {
        transform.rotation = Quat::from_rotation_z(2. * time.elapsed_secs_wrapped());
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
