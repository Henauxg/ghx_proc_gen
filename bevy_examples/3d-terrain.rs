use bevy::{
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    utils::HashMap,
};

use ghx_bevy_utilities::{pan_orbit_camera, PanOrbitCamera};
use ghx_proc_gen::{
    generator::{
        builder::GeneratorBuilder,
        node::{GeneratedNode, NodeRotation, SocketsCartesian2D},
        observer::QueuedStatefulObserver,
        rules::Rules,
        ModelSelectionHeuristic, NodeSelectionHeuristic, RngMode,
    },
    grid::{direction::Cartesian2D, GridData, GridDefinition},
};

const SCALE_FACTOR: f32 = 1. / 40.; // Models are 40 voxels wide
const MODEL_SCALE: Vec3 = Vec3::new(SCALE_FACTOR, SCALE_FACTOR, SCALE_FACTOR);

const VOID_SOCKET: u32 = 0;
const CORRIDOR: u32 = 1;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let models_asset_paths = vec![None, Some("way"), Some("wayCurve")];
    let models = vec![
        // Void
        SocketsCartesian2D::Simple(VOID_SOCKET, VOID_SOCKET, VOID_SOCKET, VOID_SOCKET).new_model(),
        // Corridor
        SocketsCartesian2D::Simple(CORRIDOR, VOID_SOCKET, CORRIDOR, VOID_SOCKET)
            .new_model()
            .with_rotation(NodeRotation::Rot90),
        // Corridor corner
        SocketsCartesian2D::Simple(VOID_SOCKET, VOID_SOCKET, CORRIDOR, CORRIDOR)
            .new_model()
            .with_all_rotations(),
    ];
    let sockets_connections = vec![(VOID_SOCKET, vec![VOID_SOCKET]), (CORRIDOR, vec![CORRIDOR])];
    let rules = Rules::new_cartesian_2d(models, sockets_connections).unwrap();
    let grid = GridDefinition::new_cartesian_2d(8, 8, false);
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_max_retry_count(10)
        .with_rng(RngMode::Seeded(1))
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();
    let mut observer = QueuedStatefulObserver::new(&mut generator);

    generator.generate_without_output().unwrap();
    observer.update();
    info!("Seed: {}", generator.get_seed());
    display_grid(observer.grid_data());

    // camera
    let translation = Vec3::new(-2.5, 4.5, 9.0);
    let radius = translation.length();
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(translation).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PanOrbitCamera {
            radius,
            ..Default::default()
        },
    ));
    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.03,
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
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

    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Circle::new(1.0).into()),
        material: materials.add(Color::WHITE.into()),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Load assets
    let mut models_assets = HashMap::new();
    for (index, path) in models_asset_paths.iter().enumerate() {
        if let Some(path) = path {
            models_assets.insert(
                index,
                asset_server.load(format!("3d_terrain/{path}.glb#Scene0")),
            );
        }
    }

    let data_grid = observer.grid_data();

    let x_offset = data_grid.grid().size_x() as f32 / 2.;
    let z_offset = data_grid.grid().size_y() as f32 / 2.;
    for z in (0..data_grid.grid().size_y()).rev() {
        for x in 0..data_grid.grid().size_x() {
            match data_grid.get_2d(x, z) {
                None => (),
                Some(node) => {
                    if let Some(asset) = models_assets.get(&node.index) {
                        commands.spawn(SceneBundle {
                            scene: asset.clone(),
                            // Y is up in Bevy.
                            transform: Transform::from_xyz(
                                (x as f32) - x_offset,
                                0.5,
                                z_offset - (z as f32),
                            )
                            .with_scale(MODEL_SCALE)
                            .with_rotation(Quat::from_rotation_y(f32::to_radians(
                                node.rotation.value() as f32,
                            ))),
                            ..default()
                        });
                    }
                }
            }
        }
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DirectionalLightShadowMap { size: 4096 });
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup);
    app.add_systems(Update, pan_orbit_camera);
    app.run();
}

fn display_grid(data_grid: &GridData<Cartesian2D, Option<GeneratedNode>>) {
    for y in (0..data_grid.grid().size_y()).rev() {
        for x in 0..data_grid.grid().size_x() {
            match data_grid.get_2d(x, y) {
                None => print!("❓"),
                Some(node) => print!("{}({}) ", node.index, node.rotation.value()),
            }
        }
        println!();
    }
}
