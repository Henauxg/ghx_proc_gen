use std::{
    io::{stdin, stdout, Write},
    thread, time,
};

use ghx_proc_gen::{
    generator::{node::GeneratedNode, observer::QueuedStatefulObserver, ModelSelectionHeuristic},
    grid::{direction::Cartesian2D, GridData},
};

use {
    ghx_proc_gen::generator::{
        builder::GeneratorBuilder, node::NodeModel, rules::Rules, NodeSelectionHeuristic, RngMode,
    },
    ghx_proc_gen::grid::Grid,
};

const ICONES: &'static [&str] = &["🗻", "🌲", "🌳", "🟩", "🟨", "🟦"]; // ,
fn main() {
    tracing_subscriber::fmt::init();
    let models = vec![
        NodeModel::new_2d(vec![3], vec![3], vec![3], vec![3]), // Mountain
        NodeModel::new_2d(vec![2, 3], vec![2, 3], vec![2, 3], vec![2, 3]).with_weight(0.5), // Forest1
        NodeModel::new_2d(vec![2, 3], vec![2, 3], vec![2, 3], vec![2, 3]).with_weight(0.5), // Forest2
        NodeModel::new_2d(vec![2, 1], vec![2, 1], vec![2, 1], vec![2, 1]), // // Meadows
        NodeModel::new_2d(vec![0, 1], vec![0, 1], vec![0, 1], vec![0, 1]), // Beach
        NodeModel::new_2d(vec![0], vec![0], vec![0], vec![0]),             // Sea
    ];
    let rules = Rules::new_cartesian_2d(models);
    let grid = Grid::new_cartesian_2d(45, 20, false);
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
        //  pause();

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
