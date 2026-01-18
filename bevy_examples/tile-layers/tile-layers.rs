use bevy::{
    app::{App, PluginGroup, Startup},
    asset::{AssetServer, Handle},
    color::Color,
    ecs::system::{Commands, Res},
    image::{Image, ImagePlugin},
    log::LogPlugin,
    math::Vec3,
    prelude::Camera2d,
    transform::components::Transform,
    utils::default,
    DefaultPlugins,
};

use bevy_examples::{plugin::ProcGenExamplesPlugin, utils::load_assets};
use bevy_ghx_proc_gen::{
    bevy_ghx_grid::{
        debug_plugin::{view::DebugGridView, DebugGridView2dBundle},
        ghx_grid::direction::Direction,
    },
    debug_plugin::generation::GenerationViewMode,
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, node_heuristic::NodeSelectionHeuristic, rules::RulesBuilder,
            ModelSelectionHeuristic, RngMode,
        },
        ghx_grid::cartesian::{coordinates::Cartesian3D, grid::CartesianGrid},
    },
    spawner_plugin::NodesSpawner,
};

use crate::rules::rules_and_assets;

mod rules;

// -----------------  Configurable values ---------------------------
/// Modify these values to control the map size.
const GRID_X: u32 = 25;
const GRID_Y: u32 = 18;

/// Modify this value to control the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::StepByStepTimed {
    steps_count: 2,
    interval_ms: 1,
};
// ------------------------------------------------------------------

const ASSETS_PATH: &str = "tile_layers";
/// Size of a block in world units (in Bevy 2d, 1 pixel is 1 world unit)
const TILE_SIZE: f32 = 32.;
/// Size of a grid node in world units
const NODE_SIZE: Vec3 = Vec3::new(TILE_SIZE, TILE_SIZE, 1.);

const ASSETS_SCALE: Vec3 = Vec3::ONE;

/// Number of z layers in the map, do not change without adapting the rules.
const GRID_Z: u32 = 5;

fn setup_scene(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d::default());
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Get rules from rules.rs
    let (assets_definitions, models, socket_collection) = rules_and_assets();

    let rules = RulesBuilder::new_cartesian_3d(models, socket_collection)
        // Use ZForward as the up axis (rotation axis for models) since we are using Bevy in 2D
        .with_rotation_axis(Direction::ZForward)
        .build()
        .unwrap();
    let grid = CartesianGrid::new_cartesian_3d(GRID_X, GRID_Y, GRID_Z, false, false, false);
    let mut gen_builder = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability);
    let gen_observer = gen_builder.add_queued_observer();
    let generator = gen_builder.build().unwrap();

    let models_assets = load_assets::<Image>(&asset_server, assets_definitions, ASSETS_PATH, "png");

    commands.spawn((
        Transform::from_translation(Vec3 {
            x: -TILE_SIZE * grid.size_x() as f32 / 2.,
            y: -TILE_SIZE * grid.size_y() as f32 / 2.,
            z: 0.,
        }),
        grid,
        generator,
        gen_observer,
        NodesSpawner::new(models_assets, NODE_SIZE, Vec3::ZERO).with_z_offset_from_y(true),
        DebugGridView2dBundle {
            view: DebugGridView::new(false, true, Color::WHITE, NODE_SIZE),
            ..default()
        },
    ));
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(LogPlugin {
                filter: "info,wgpu_core=error,wgpu_hal=error,ghx_proc_gen=debug".into(),
                level: bevy::log::Level::DEBUG,
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        ProcGenExamplesPlugin::<Cartesian3D, Handle<Image>>::new(
            GENERATION_VIEW_MODE,
            ASSETS_SCALE,
        ),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene));
    app.run();
}
