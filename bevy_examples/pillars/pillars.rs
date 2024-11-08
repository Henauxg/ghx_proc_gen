use std::{
    f32::consts::{FRAC_PI_2, PI},
    sync::Arc,
};

use bevy::{
    color::palettes::css::{GRAY, ORANGE_RED},
    log::LogPlugin,
    pbr::DirectionalLightShadowMap,
    prelude::*,
};

use bevy_examples::{plugin::ProcGenExamplesPlugin, utils::load_assets};

use bevy_ghx_proc_gen::{
    bevy_ghx_grid::debug_plugin::{view::DebugGridView, DebugGridView3dBundle},
    gen::{
        assets::{AssetSpawner, RulesModelsAssets},
        debug_plugin::GenerationViewMode,
    },
    proc_gen::{
        generator::{builder::GeneratorBuilder, rules::RulesBuilder},
        ghx_grid::cartesian::{coordinates::Cartesian3D, grid::CartesianGrid},
    },
    GeneratorBundle,
};
use bevy_ghx_utils::camera::{update_pan_orbit_camera, PanOrbitCameraBundle, PanOrbitState};

use crate::rules::rules_and_assets;

mod rules;

// --------------------------------------------
/// Change this value to change the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

/// Change this to change the map size.
const GRID_HEIGHT: u32 = 7;
const GRID_X: u32 = 30;
const GRID_Z: u32 = 70;
// --------------------------------------------

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
    // Camera
    let camera_position = Vec3::new(0., 3. * GRID_HEIGHT as f32, 0.75 * GRID_Z as f32);
    let radius = camera_position.length();
    commands.spawn((
        PanOrbitCameraBundle {
            camera: Camera3dBundle {
                transform: Transform::from_translation(camera_position)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            state: PanOrbitState {
                radius,
                pitch: -FRAC_PI_2 / 2.,
                ..Default::default()
            },
            ..Default::default()
        },
        FogSettings {
            color: Color::srgba(0.2, 0.15, 0.1, 1.0),
            falloff: FogFalloff::Linear {
                start: 55.0,
                end: 145.0,
            },
            ..default()
        },
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Plane3d::default())),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.21, 0.21, 0.21),
                ..default()
            }),
            transform: Transform::from_scale(Vec3::splat(10000.0)).with_translation(Vec3::new(
                0.,
                BLOCK_SIZE / 2.,
                0.,
            )),
            ..default()
        },
        Name::new("Ground"),
    ));

    // Scene lights
    commands.insert_resource(AmbientLight {
        color: Color::Srgba(ORANGE_RED),
        brightness: 0.05,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 3000.,
            color: Color::srgb(1.0, 0.85, 0.65),
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 0.0),
            rotation: Quat::from_euler(EulerRot::ZYX, 0., -PI / 5., -PI / 3.),
            ..default()
        },
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            illuminance: 1250.,
            color: Color::srgb(1.0, 0.85, 0.65),
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

    let models_assets: RulesModelsAssets<Handle<Scene>> =
        load_assets(&asset_server, models_asset_paths, "pillars", "glb#Scene0");
    let asset_spawner = AssetSpawner::new(
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
            GeneratorBundle {
                spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3 {
                    x: (grid.size_x() as f32) * (i as f32 - 1.),
                    y: 0.,
                    z: -(grid.size_z() as f32) * 0.5,
                })),
                grid: grid.clone(),
                generator,
                // We also share the RulesModelsAssets between all the generators
                asset_spawner: asset_spawner.clone(),
            },
            observer,
            DebugGridView3dBundle {
                view: DebugGridView::new(false, true, Color::Srgba(GRAY), NODE_SIZE),
                ..default()
            },
            Name::new(format!("Grid_{}", i)),
        ));
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins((
        DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,ghx_proc_gen=debug".into(),
            level: bevy::log::Level::DEBUG,
            ..default()
        }),
        ProcGenExamplesPlugin::<Cartesian3D, Handle<Scene>>::new(
            GENERATION_VIEW_MODE,
            ASSETS_SCALE,
        ),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene))
        .add_systems(Update, update_pan_orbit_camera);

    app.run();
}
