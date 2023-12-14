use std::{collections::HashMap, time::Duration};

use bevy::{log::LogPlugin, prelude::*};

use bevy_examples::{
    spawn_node, step_by_step_input_update, step_by_step_timed_update, Generation, GenerationTimer,
    GenerationViewMode,
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

/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

const GRID_X: u32 = 20;
const GRID_Y: u32 = 20;

/// Size of a block in world units
const TILE_SIZE: f32 = 32.;
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
                    TILE_SIZE,
                );
            }
        }
    }

    commands.insert_resource(Generation {
        models_assets,
        gen: generator,
        observer,
        node_size: TILE_SIZE,
    });

    commands.spawn((
        SpatialBundle::from_transform(Transform::from_translation(Vec3 {
            x: 0.,
            y: 0.,
            z: 0.,
        })),
        Grid { def: grid },
    ));
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                filter: "info,wgpu_core=warn,wgpu_hal=warn,ghx_proc_gen=debug".into(),
                level: bevy::log::Level::DEBUG,
            })
            .set(ImagePlugin::default_nearest()),
    );
    app.add_systems(Startup, (setup_generator, setup_scene));

    match GENERATION_VIEW_MODE {
        GenerationViewMode::StepByStep(_) => {
            app.add_systems(Update, step_by_step_timed_update::<Cartesian2D>);
        }
        GenerationViewMode::StepByStepPaused => {
            app.add_systems(Update, step_by_step_input_update::<Cartesian2D>);
        }
        GenerationViewMode::Final => (),
    };

    app.run();
}
