use std::{
    io::{stdin, stdout, Write},
    thread, time,
};

use ghx_proc_gen::{
    generator::{
        node::{GeneratedNode, SocketsCartesian2D},
        observer::QueuedStatefulObserver,
        ModelSelectionHeuristic,
    },
    grid::{direction::Cartesian2D, GridData},
};

use {
    ghx_proc_gen::generator::{
        builder::GeneratorBuilder, rules::Rules, NodeSelectionHeuristic, RngMode,
    },
    ghx_proc_gen::grid::Grid,
};

const ICONES: &'static [&str] = &["🗻", "🌲", "🌳", "🟩", "🟨", "🟦"]; // ,

const MOUNTAIN: u32 = 0;
const FOREST: u32 = 1;
const MEADOWS: u32 = 2;
const BEACH: u32 = 3;
const SEA: u32 = 4;

fn main() {
    tracing_subscriber::fmt::init();

    let models = vec![
        SocketsCartesian2D::Mono(MOUNTAIN).new_model(),
        SocketsCartesian2D::Mono(FOREST)
            .new_model()
            .with_weight(0.5), // Variation 1
        SocketsCartesian2D::Mono(FOREST)
            .new_model()
            .with_weight(0.5), // Variation 2
        SocketsCartesian2D::Mono(MEADOWS).new_model(),
        SocketsCartesian2D::Mono(BEACH).new_model(),
        SocketsCartesian2D::Mono(SEA).new_model(),
    ];
    let sockets_connections = vec![
        (MOUNTAIN, vec![MOUNTAIN, FOREST]),
        (FOREST, vec![FOREST, MEADOWS]),
        (MEADOWS, vec![MEADOWS, BEACH]),
        (BEACH, vec![BEACH, SEA]),
        (SEA, vec![SEA]),
    ];
    let rules = Rules::new_cartesian_2d(models, sockets_connections);
    let grid = Grid::new_cartesian_2d(30, 15, false);
    let size = grid.total_size();
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_max_retry_count(750)
        .with_rng(RngMode::Random)
        .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();
    let mut observer = QueuedStatefulObserver::new(&mut generator);

    for i in 1..=size {
        generator.select_and_propagate().unwrap();
        observer.update();
        println!("Grid at step {}:", i);
        display_grid(observer.grid_data());
        // pause();
        // thread::sleep(time::Duration::from_millis(400));
    }
}

fn display_grid(data_grid: &GridData<Cartesian2D, Option<GeneratedNode>>) {
    for y in (0..data_grid.grid().size_y()).rev() {
        for x in 0..data_grid.grid().size_x() {
            match data_grid.get_2d(x, y) {
                None => print!("❓"),
                Some(node) => print!("{}", ICONES[node.index]),
            }
        }
        println!();
    }
}

fn pause() {
    let mut word = String::new();
    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue").unwrap();
    stdout.flush().unwrap();
    stdin().read_line(&mut word).unwrap();
}
