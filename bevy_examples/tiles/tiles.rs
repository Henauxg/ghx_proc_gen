use std::collections::HashMap;

use bevy::{log::LogPlugin, prelude::*};

use bevy_examples::{
    anim::{ease_in_cubic, SpawningScaleAnimation},
    plugin::{sprite_node_spawner, ProcGenExamplesPlugin},
    Generation, GenerationViewMode,
};
use bevy_ghx_proc_gen::{
    grid::Grid,
    proc_gen::{
        generator::{
            builder::GeneratorBuilder, observer::QueuedObserver, rules::RulesBuilder,
            ModelSelectionHeuristic, NodeSelectionHeuristic, RngMode,
        },
        grid::{direction::Cartesian2D, GridDefinition},
    },
};
use bevy_ghx_utilities::camera::PanOrbitCamera;

use crate::rules::rules_and_assets;

mod rules;

// --------------------------------------------
/// Change this to change the map size.
const GRID_X: u32 = 20;
const GRID_Y: u32 = 20;

/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::StepByStep(15);
// --------------------------------------------

/// Size of a block in world units
const TILE_SIZE: f32 = 32.;
const NODE_SIZE: Vec3 = Vec3::new(TILE_SIZE, TILE_SIZE, 1.);
const ASSETS_PATH: &str = "tiles";

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
            models_assets.insert(
                index,
                asset_server.load(format!("{ASSETS_PATH}/{path}.png")),
            );
        }
    }

    let grid_entity = commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_translation(Vec3 {
                x: -TILE_SIZE * grid.size_x() as f32 / 2.,
                y: -TILE_SIZE * grid.size_y() as f32 / 2.,
                z: 0.,
            })),
            Grid { def: grid },
        ))
        .id();

    commands.insert_resource(Generation {
        models_assets,
        gen: generator,
        observer,
        node_scale: NODE_SIZE,
        grid_entity,

        assets_initial_scale: Vec3::ZERO,
        bundle_spawner: sprite_node_spawner,
        spawn_animation: Some(SpawningScaleAnimation::new(0.4, Vec3::ONE, ease_in_cubic)),
    });
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
        ProcGenExamplesPlugin::<Cartesian2D, Image, SpriteBundle>::new(GENERATION_VIEW_MODE),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene));

    app.run();
}
