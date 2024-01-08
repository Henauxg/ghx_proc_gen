use std::{
    io::{stdin, stdout, Write},
    thread, time,
};

use ghx_proc_gen::{
    generator::{
        model::ModelInstance,
        node_heuristic::NodeSelectionHeuristic,
        observer::QueuedStatefulObserver,
        rules::RulesBuilder,
        socket::{SocketCollection, SocketsCartesian2D},
        GenerationStatus, ModelSelectionHeuristic,
    },
    grid::{direction::Cartesian2D, GridData},
};

use {
    ghx_proc_gen::generator::{builder::GeneratorBuilder, RngMode},
    ghx_proc_gen::grid::GridDefinition,
};

pub enum GenerationViewMode {
    StepByStep(u64),
    StepByStepPaused,
    Final,
}

const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mut sockets = SocketCollection::new();
    let mountain = sockets.create();
    let forest = sockets.create();
    let meadows = sockets.create();
    let beach = sockets.create();
    let sea = sockets.create();
    let deep_sea = sockets.create();

    let icons_and_models = vec![
        ("🗻", SocketsCartesian2D::Mono(mountain).new_model()),
        (
            "🌲", // Variation 1
            SocketsCartesian2D::Mono(forest)
                .new_model()
                .with_weight(0.5),
        ),
        (
            "🌳", // Variation 2
            SocketsCartesian2D::Mono(forest)
                .new_model()
                .with_weight(0.5),
        ),
        ("🟩", SocketsCartesian2D::Mono(meadows).new_model()),
        ("🟨", SocketsCartesian2D::Mono(beach).new_model()),
        ("🟦", SocketsCartesian2D::Mono(sea).new_model()),
        (
            "🟦",
            SocketsCartesian2D::Mono(deep_sea)
                .new_model()
                .with_weight(2.),
        ),
    ];

    sockets.add_connections(vec![
        (mountain, vec![mountain, forest]),
        (forest, vec![forest, meadows]),
        (meadows, vec![meadows, beach]),
        (beach, vec![beach, sea]),
        (sea, vec![sea]),
        (deep_sea, vec![sea]),
    ]);

    let icons: Vec<&'static str> = icons_and_models.iter().map(|t| t.0).collect();
    let models = icons_and_models.iter().map(|t| t.1.clone()).collect();

    let rules = RulesBuilder::new_cartesian_2d(models, sockets)
        .build()
        .unwrap();
    let grid = GridDefinition::new_cartesian_2d(35, 12, false, false);
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
            generator.generate().unwrap();
            observer.dequeue_all();
            println!("Final grid:");
            display_grid(observer.grid_data(), &icons);
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
                display_grid(observer.grid_data(), &icons);
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

fn display_grid(
    data_grid: &GridData<Cartesian2D, Option<ModelInstance>>,
    icons: &Vec<&'static str>,
) {
    for y in (0..data_grid.grid().size_y()).rev() {
        for x in 0..data_grid.grid().size_x() {
            match data_grid.get_2d(x, y) {
                None => print!("❓"),
                Some(node) => print!("{}", icons[node.model_index]),
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
