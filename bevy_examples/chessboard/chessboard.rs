use bevy::prelude::*;

use bevy_ghx_grid::ghx_grid::{
    coordinate_system::Cartesian2D,
    grid::{GridDefinition, GridPosition},
};
use bevy_ghx_proc_gen::{
    gen::{
        assets::{AssetSpawner, RulesModelsAssets},
        default_bundles::PbrMesh,
        simple_plugin::ProcGenSimplePlugin,
    },
    proc_gen::generator::{
        builder::GeneratorBuilder,
        model::ModelCollection,
        rules::RulesBuilder,
        socket::{SocketCollection, SocketsCartesian2D},
    },
    GeneratorBundle,
};

const CUBE_SIZE: f32 = 1.;
const NODE_SIZE: Vec3 = Vec3::splat(CUBE_SIZE);

fn setup_scene(mut commands: Commands) {
    // Camera
    commands.spawn((Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(0., -11., 6.))
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    },));

    // Scene lights
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 5500.,
            ..default()
        },
        ..default()
    });
}

fn setup_generator(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A SocketCollection is what we use to create sockets and define their connections
    let mut sockets = SocketCollection::new();
    // For this example, we will only need two sockets
    let (white, black) = (sockets.create(), sockets.create());

    // With the following, a white socket can connect to a black socket and vice-versa
    sockets.add_connection(white, vec![black]);

    let mut models = ModelCollection::<Cartesian2D>::new();
    // We define 2 very simple models, a white tile model with the `white` socket on each side and a black tile model with the `black` socket on each side
    models.create(SocketsCartesian2D::Mono(white));
    // We keep the black model for later
    let black_model = models.create(SocketsCartesian2D::Mono(black)).clone();

    // We give the models and socket collection to a RulesBuilder and get our Rules
    let rules = RulesBuilder::new_cartesian_2d(models, sockets)
        .build()
        .unwrap();

    // Like a chess board, let's do an 8x8 2d grid
    let grid = GridDefinition::new_cartesian_2d(8, 8, false, false);

    // There many more parameters you can tweak on a Generator before building it, explore the API.
    let generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid.clone())
        // Let's ensure that we make a chessboard, with a black square bottom-left
        .with_initial_nodes(vec![(GridPosition::new_xy(0, 0), black_model)])
        .unwrap()
        .build()
        .unwrap();

    // Create our assets. We define them in a separate collection for the sake of simplicity
    let cube_mesh = meshes.add(Mesh::from(Cuboid {
        half_size: Vec3::splat(CUBE_SIZE / 2.),
    }));
    let white_mat = materials.add(Color::WHITE);
    let black_mat = materials.add(Color::BLACK);
    let mut models_assets = RulesModelsAssets::<PbrMesh>::new();
    models_assets.add_asset(
        0,
        PbrMesh {
            mesh: cube_mesh.clone(),
            material: white_mat,
        },
    );
    models_assets.add_asset(
        1,
        PbrMesh {
            mesh: cube_mesh.clone(),
            material: black_mat,
        },
    );

    // Add the GeneratorBundle, the plugin will generate and spawn the nodes
    commands.spawn(GeneratorBundle {
        spatial: SpatialBundle::from_transform(Transform::from_translation(Vec3::new(
            -4., -4., 0.,
        ))),
        grid,
        generator,
        asset_spawner: AssetSpawner::new(models_assets, NODE_SIZE, Vec3::ONE),
    });
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        ProcGenSimplePlugin::<Cartesian2D, PbrMesh>::new(),
    ));
    app.add_systems(Startup, (setup_generator, setup_scene));
    app.run();
}
