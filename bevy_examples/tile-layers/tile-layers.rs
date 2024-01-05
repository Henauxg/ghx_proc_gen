use bevy::{app::PluginGroup, log::LogPlugin, prelude::*};

use bevy_examples::{
    anim::{ease_in_cubic, SpawningScaleAnimation},
    plugin::{sprite_node_spawner, ProcGenExamplesPlugin},
    utils::load_assets,
    Generation, GenerationControl, GenerationViewMode,
};
use bevy_ghx_proc_gen::{
    grid::{
        view::{DebugGridView, DebugGridView2d, DebugGridViewConfig2d},
        Grid,
    },
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, rules::RulesBuilder, ModelSelectionHeuristic,
            NodeSelectionHeuristic, RngMode,
        },
        grid::{
            direction::{Cartesian3D, Direction},
            GridDefinition,
        },
    },
};

use crate::rules::rules_and_assets;

mod rules;

// --------------------------------------------
/// Change this to change the map size.
const GRID_X: u32 = 20;
const GRID_Y: u32 = 20;

/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::StepByStep(1);
// --------------------------------------------

/// Size of a block in world units (in Bevy 2d, 1 pixel is 1 world unit)
const TILE_SIZE: f32 = 32.;
const NODE_SIZE: Vec3 = Vec3::new(TILE_SIZE, TILE_SIZE, 1.);
const ASSETS_PATH: &str = "tile_layers";
/// Number of z layers in the map, do not chnage without adapting the rules.
const GRID_Z: u32 = 5;

fn setup_scene(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2dBundle::default());
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Get rules from rules.rs
    let (assets_definitions, models, socket_collection) = rules_and_assets();

    let rules = RulesBuilder::new_cartesian_3d(models, socket_collection)
        // Use ZForward as the up axis (rotation axis for models) since we are using Bevy in 2D
        .with_rotation_axis(Direction::ZForward)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_3d(GRID_X, GRID_Y, GRID_Z, false, false, false);
    let gen = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();

    let models_assets = load_assets(&asset_server, assets_definitions, ASSETS_PATH, "png");

    let grid_entity = commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_translation(Vec3 {
                x: -TILE_SIZE * grid.size_x() as f32 / 2.,
                y: -TILE_SIZE * grid.size_y() as f32 / 2.,
                z: 0.,
            })),
            Grid { def: grid },
            DebugGridView2d {
                config: DebugGridViewConfig2d {
                    node_size: Vec2::splat(TILE_SIZE),
                    ..Default::default()
                },
                view: DebugGridView::new(false, true),
            },
        ))
        .id();

    commands.insert_resource(Generation::new(
        models_assets,
        gen,
        NODE_SIZE,
        grid_entity,
        Vec3::ZERO,
        sprite_node_spawner,
        Some(SpawningScaleAnimation::new(0.4, Vec3::ONE, ease_in_cubic)),
        true,
    ));

    commands.insert_resource(GenerationControl::new(true, true, true));
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
        ProcGenExamplesPlugin::<Cartesian3D, Image, SpriteBundle>::new(GENERATION_VIEW_MODE),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene));
    app.run();
}
