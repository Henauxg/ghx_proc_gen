use bevy::{
    app::{App, Startup},
    ecs::system::Commands,
    DefaultPlugins,
};
use ghx_proc_gen::{
    generator::{
        node::{GeneratedNode, NodeRotation, SocketsCartesian2D},
        observer::QueuedStatefulObserver,
        ModelSelectionHeuristic,
    },
    grid::{direction::Cartesian2D, GridData},
};

use {
    ghx_proc_gen::generator::{
        builder::GeneratorBuilder, rules::Rules, NodeSelectionHeuristic, RngMode,
    },
    ghx_proc_gen::grid::GridDefinition,
};

const VOID_SOCKET: u32 = 0;
const CORRIDOR: u32 = 1;

fn setup(commands: Commands) {
    let meshes = &[None, Some("way"), Some("wayCurve")];
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
    let grid = GridDefinition::new_cartesian_2d(4, 4, false);
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
    println!("Seed: {}", generator.get_seed());
    display_grid(observer.grid_data());
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup);
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
