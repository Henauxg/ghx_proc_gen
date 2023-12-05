use std::{
    io::{stdin, stdout, Write},
    thread, time,
};

use ghx_proc_gen::{
    generator::{
        node::{GeneratedNode, SocketsCartesian2D},
        observer::QueuedStatefulObserver,
        rules::RulesBuilder,
        GenerationStatus, ModelSelectionHeuristic,
    },
    grid::{direction::Cartesian2D, GridData},
};

use {
    ghx_proc_gen::generator::{builder::GeneratorBuilder, NodeSelectionHeuristic, RngMode},
    ghx_proc_gen::grid::GridDefinition,
};

pub enum GenerationViewMode {
    StepByStep(u64),
    StepByStepPaused,
    Final,
}

const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

const ICONES: &'static [&str] = &["🗻", "🌲", "🌳", "🟩", "🟨", "🟦", "🟦"];

const MOUNTAIN: u32 = 0;
const FOREST: u32 = 1;
const MEADOWS: u32 = 2;
const BEACH: u32 = 3;
const SEA: u32 = 4;
const DEEP_SEA: u32 = 5;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

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
        SocketsCartesian2D::Mono(DEEP_SEA)
            .new_model()
            .with_weight(2.),
    ];
    let sockets_connections = vec![
        (MOUNTAIN, vec![MOUNTAIN, FOREST]),
        (FOREST, vec![FOREST, MEADOWS]),
        (MEADOWS, vec![MEADOWS, BEACH]),
        (BEACH, vec![BEACH, SEA]),
        (SEA, vec![SEA]),
        (DEEP_SEA, vec![DEEP_SEA, SEA]),
    ];
    let rules = RulesBuilder::new_cartesian_2d(models, sockets_connections)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_2d(35, 12, false);
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_max_retry_count(10)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::Random)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build();
    let mut observer = QueuedStatefulObserver::new(&mut generator);

    match GENERATION_VIEW_MODE {
        GenerationViewMode::Final => {
            generator.generate_without_output().unwrap();
            observer.dequeue_all();
            println!("Final grid:");
            display_grid(observer.grid_data());
        }
        _ => {
            let mut step = 0;
            let mut done = false;
            while !done {
                match generator.select_and_propagate() {
                    Ok(status) => match status {
                        GenerationStatus::Ongoing => (),
                        GenerationStatus::Done => done = true,
                    },
                    Err(_) => (),
                }
                observer.dequeue_all();
                println!("Grid at iteration n°{}:", step);
                display_grid(observer.grid_data());
                match GENERATION_VIEW_MODE {
                    GenerationViewMode::StepByStep(delay) => {
                        thread::sleep(time::Duration::from_millis(delay));
                    }
                    GenerationViewMode::StepByStepPaused => pause(),
                    _ => (),
                }
                step += 1;
            }
        }
    }
}

fn display_grid(data_grid: &GridData<Cartesian2D, Option<GeneratedNode>>) {
    for y in (0..data_grid.grid().size_y()).rev() {
        for x in 0..data_grid.grid().size_x() {
            match data_grid.get_2d(x, y) {
                None => print!("❓"),
                Some(node) => print!("{}", ICONES[node.model_index]),
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
