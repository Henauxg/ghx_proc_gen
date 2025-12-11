use bevy::{
    app::{App, PluginGroup, Startup, Update},
    asset::{AssetServer, Handle},
    color::Color,
    ecs::{
        entity::Entity,
        message::MessageWriter,
        query::With,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res},
    },
    image::{Image, ImagePlugin},
    input::{common_conditions::input_just_pressed, keyboard::KeyCode},
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
        debug_plugin::{markers::MarkerDespawnEvent, view::DebugGridView, DebugGridView2dBundle},
        ghx_grid::direction::Direction,
    },
    debug_plugin::{
        cursor::{
            spawn_marker_and_create_cursor, Cursor, CursorMarkerSettings, SelectCursor,
            SelectionCursorMarkerSettings,
        },
        egui_editor::{BrushEvent, ModelBrush},
        generation::GenerationViewMode,
    },
    proc_gen::{
        generator::{
            builder::GeneratorBuilder,
            model::{ModelInstance, ModelRotation},
            node_heuristic::NodeSelectionHeuristic,
            rules::{ModelInfo, RulesBuilder},
            ModelSelectionHeuristic, RngMode,
        },
        ghx_grid::cartesian::{
            coordinates::{Cartesian3D, CartesianPosition},
            grid::CartesianGrid,
        },
    },
    spawner_plugin::NodesSpawner,
};

use crate::rules::rules_and_assets;

mod rules;

// -----------------  Configurable values ---------------------------
/// Modify these values to control the map size.
const GRID_X: u32 = 30;
const GRID_Y: u32 = 25;

/// Modify this value to control the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::StepByStepTimed {
    steps_count: 4,
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

    app.add_systems(
        Update,
        (
            demo_force_brush_water.run_if(input_just_pressed(KeyCode::KeyG)),
            demo_force_brush_tree.run_if(input_just_pressed(KeyCode::KeyH)),
        ),
    );

    app.run();
}

pub fn demo_force_brush_windmill(mut brush_events: MessageWriter<BrushEvent>) {
    brush_events.write(BrushEvent::UpdateBrush(ModelBrush {
        info: ModelInfo {
            weight: 0.5,
            name: "Windmill".into(),
        },
        instance: ModelInstance {
            model_index: 13,
            rotation: ModelRotation::Rot0,
        },
    }));
}

const WATER_LAYER_Z: u32 = 3;
pub fn demo_force_brush_water(
    mut commands: Commands,
    mut brush_events: MessageWriter<BrushEvent>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    grids: Query<(Entity, &CartesianGrid<Cartesian3D>)>,
    mut marker_events: MessageWriter<MarkerDespawnEvent>,
    selection_marker_settings: Res<SelectionCursorMarkerSettings>,
) {
    let Ok(mut cursor) = selection_cursor.single_mut() else {
        return;
    };

    match cursor.0.as_mut() {
        Some(grid_cursor) => {
            let Ok((_grid_entity, grid)) = grids.get(grid_cursor.grid) else {
                return;
            };
            marker_events.write(MarkerDespawnEvent::Marker(grid_cursor.marker));
            grid_cursor.position.z = WATER_LAYER_Z;
            grid_cursor.index = grid.index_from_pos(&grid_cursor.position);
        }
        None => {
            // Currently no selection cursor, spawn it on the last Grid
            let Some((grid_entity, grid)) = grids.iter().last() else {
                return;
            };

            let pos = CartesianPosition::new(0, 0, WATER_LAYER_Z);
            cursor.0 = Some(spawn_marker_and_create_cursor(
                &mut commands,
                grid_entity,
                pos,
                grid.index_from_pos(&pos),
                selection_marker_settings.color(),
            ));
        }
    };

    brush_events.write(BrushEvent::UpdateBrush(ModelBrush {
        info: ModelInfo {
            weight: 0.5,
            name: "Water".into(),
        },
        instance: ModelInstance {
            model_index: 30,
            rotation: ModelRotation::Rot0,
        },
    }));
}

const TREE_LAYER_Z: u32 = 4;
pub fn demo_force_brush_tree(
    mut commands: Commands,
    mut brush_events: MessageWriter<BrushEvent>,
    mut selection_cursor: Query<&mut Cursor, With<SelectCursor>>,
    grids: Query<(Entity, &CartesianGrid<Cartesian3D>)>,
    mut marker_events: MessageWriter<MarkerDespawnEvent>,
    selection_marker_settings: Res<SelectionCursorMarkerSettings>,
) {
    let Ok(mut cursor) = selection_cursor.single_mut() else {
        return;
    };

    match cursor.0.as_mut() {
        Some(grid_cursor) => {
            let Ok((_grid_entity, grid)) = grids.get(grid_cursor.grid) else {
                return;
            };
            marker_events.write(MarkerDespawnEvent::Marker(grid_cursor.marker));
            grid_cursor.position.z = TREE_LAYER_Z;
            grid_cursor.index = grid.index_from_pos(&grid_cursor.position);
        }
        None => {
            // Currently no selection cursor, spawn it on the last Grid
            let Some((grid_entity, grid)) = grids.iter().last() else {
                return;
            };

            let pos = CartesianPosition::new(0, 0, TREE_LAYER_Z);
            cursor.0 = Some(spawn_marker_and_create_cursor(
                &mut commands,
                grid_entity,
                pos,
                grid.index_from_pos(&pos),
                selection_marker_settings.color(),
            ));
        }
    };

    brush_events.write(BrushEvent::UpdateBrush(ModelBrush {
        info: ModelInfo {
            weight: 0.5,
            name: "Tree".into(),
        },
        instance: ModelInstance {
            model_index: 45,
            rotation: ModelRotation::Rot0,
        },
    }));
}
