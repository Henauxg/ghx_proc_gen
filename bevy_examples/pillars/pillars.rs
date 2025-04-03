use std::{f32::consts::PI, sync::Arc};

use bevy::{
    app::{App, Startup},
    asset::{AssetServer, Assets, Handle},
    color::{
        palettes::css::{GRAY, ORANGE_RED},
        Color,
    },
    core::Name,
    core_pipeline::core_3d::Camera3d,
    log::LogPlugin,
    math::{EulerRot, Quat, Vec3},
    pbr::{
        AmbientLight, DirectionalLight, DirectionalLightShadowMap, DistanceFog, FogFalloff,
        MeshMaterial3d, StandardMaterial,
    },
    prelude::{Commands, Mesh, Mesh3d, Plane3d, PluginGroup, Res, ResMut, Transform},
    scene::Scene,
    utils::default,
    DefaultPlugins,
};

use bevy_editor_cam::{prelude::EditorCam, DefaultEditorCamPlugins};
use bevy_examples::{plugin::ProcGenExamplesPlugin, utils::load_assets};

use bevy_ghx_proc_gen::{
    assets::ModelsAssets,
    bevy_ghx_grid::debug_plugin::{view::DebugGridView, DebugGridView3dBundle},
    debug_plugin::generation::GenerationViewMode,
    proc_gen::{
        generator::{builder::GeneratorBuilder, rules::RulesBuilder},
        ghx_grid::cartesian::{coordinates::Cartesian3D, grid::CartesianGrid},
    },
    spawner_plugin::NodesSpawner,
};

use crate::rules::rules_and_assets;

mod rules;

// -----------------  Configurable values ---------------------------
/// Modify this value to control the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

/// Modify these values to control the map size.
const GRID_HEIGHT: u32 = 7;
const GRID_X: u32 = 30;
const GRID_Z: u32 = 70;
// ------------------------------------------------------------------

/// Size of a block in world units
const BLOCK_SIZE: f32 = 1.;
const NODE_SIZE: Vec3 = Vec3::splat(BLOCK_SIZE);

const ASSETS_SCALE_FACTOR: f32 = BLOCK_SIZE / 4.; // Models are 4 units wide
const ASSETS_SCALE: Vec3 = Vec3::splat(ASSETS_SCALE_FACTOR);

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let camera_position = Vec3::new(0., 3. * GRID_HEIGHT as f32, 0.75 * GRID_Z as f32);
    commands.spawn((
        Name::new("Camera"),
        Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
        Camera3d::default(),
        EditorCam::default(),
        DistanceFog {
            color: Color::srgba(0.2, 0.15, 0.1, 1.0),
            falloff: FogFalloff::Linear {
                start: 55.0,
                end: 145.0,
            },
            ..default()
        },
    ));
    commands.spawn((
        Name::new("Ground plane"),
        Transform::from_scale(Vec3::splat(10000.0)).with_translation(Vec3::new(
            0.,
            BLOCK_SIZE / 2.,
            0.,
        )),
        Mesh3d(meshes.add(Mesh::from(Plane3d::default()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.21, 0.21, 0.21),
            ..default()
        })),
    ));

    // Scene lights
    commands.insert_resource(AmbientLight {
        color: Color::Srgba(ORANGE_RED),
        brightness: 0.05,
    });
    commands.spawn((
        Name::new("Main light"),
        Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0., -PI / 5., -PI / 3.),
            ..default()
        },
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 3000.,
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
            illuminance: 1250.,
            color: Color::srgb(1.0, 0.85, 0.65),
            ..default()
        },
    ));
}

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Get rules from rules.rs
    let (models_asset_paths, models, socket_collection) = rules_and_assets();

    let rules = Arc::new(
        RulesBuilder::new_cartesian_3d(models, socket_collection)
            .build()
            .unwrap(),
    );
    let grid = CartesianGrid::new_cartesian_3d(GRID_X, GRID_HEIGHT, GRID_Z, false, false, false);
    let gen_builder = GeneratorBuilder::new()
        // We share the Rules between all the generators
        .with_shared_rules(rules.clone())
        .with_grid(grid.clone());

    let models_assets: ModelsAssets<Handle<Scene>> =
        load_assets(&asset_server, models_asset_paths, "pillars", "glb#Scene0");
    let node_spawner = NodesSpawner::new(
        models_assets,
        NODE_SIZE,
        // We spawn assets with a scale of 0 since we animate their scale in the examples
        Vec3::ZERO,
    );

    for i in 0..=1 {
        let mut gen_builder = gen_builder.clone();
        let observer = gen_builder.add_queued_observer();
        let generator = gen_builder.build().unwrap();

        commands.spawn((
            Name::new(format!("Grid n°{}", i)),
            Transform::from_translation(Vec3 {
                x: (grid.size_x() as f32) * (i as f32 - 1.),
                y: 0.,
                z: -(grid.size_z() as f32) * 0.5,
            }),
            grid.clone(),
            generator,
            observer,
            // We also share the ModelsAssets between all the generators
            node_spawner.clone(),
            DebugGridView3dBundle {
                view: DebugGridView::new(false, true, Color::Srgba(GRAY), NODE_SIZE),
                ..default()
            },
        ));
    }
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
        DefaultEditorCamPlugins,
        ProcGenExamplesPlugin::<Cartesian3D, Handle<Scene>>::new(
            GENERATION_VIEW_MODE,
            ASSETS_SCALE,
        ),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene));

    app.run();
}
