use std::{
    io::{stdin, stdout, Write},
    thread, time,
};

use ghx_proc_gen::{
    generator::{
        model::{ModelCollection, ModelInstance},
        node_heuristic::NodeSelectionHeuristic,
        observer::QueuedStatefulObserver,
        rules::RulesBuilder,
        socket::{SocketCollection, SocketsCartesian2D},
        GenerationStatus, ModelSelectionHeuristic,
    },
    ghx_grid::{
        coordinate_system::Cartesian2D,
        grid::{GridData, CartesianGrid},
    },
};

use ghx_proc_gen::generator::{builder::GeneratorBuilder, RngMode};

pub enum GenerationViewMode {
    /// The parameter is the number of milliseconds to wait between each step.
    StepByStepTimed(u64),
    StepByStepPaused,
    Final,
}

// Change this to change how the generation advancess
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

    let mut models = ModelCollection::<Cartesian2D>::new();
    let mut icons = Vec::new();

    icons.push("🗻");
    models.create(SocketsCartesian2D::Mono(mountain));

    icons.push("🌲"); // Variation 1
    models
        .create(SocketsCartesian2D::Mono(forest))
        .with_weight(0.5);

    icons.push("🌳"); // Variation 2
    models
        .create(SocketsCartesian2D::Mono(forest))
        .with_weight(0.5);

    icons.push("🟩");
    models.create(SocketsCartesian2D::Mono(meadows));

    icons.push("🟨");
    models.create(SocketsCartesian2D::Mono(beach));

    icons.push("🟦");
    models.create(SocketsCartesian2D::Mono(sea));

    icons.push("🟦");
    models
        .create(SocketsCartesian2D::Mono(deep_sea))
        .with_weight(2.);

    sockets.add_connections(vec![
        (mountain, vec![mountain, forest]),
        (forest, vec![forest, meadows]),
        (meadows, vec![meadows, beach]),
        (beach, vec![beach, sea]),
        (sea, vec![sea]),
        (deep_sea, vec![sea]),
    ]);

    let rules = RulesBuilder::new_cartesian_2d(models, sockets)
        .build()
        .unwrap();
    let grid = CartesianGrid::new_cartesian_2d(35, 12, false, false);
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        .with_max_retry_count(10)
        .with_rng(RngMode::RandomSeed)
        .with_node_heuristic(NodeSelectionHeuristic::Random)
        .with_model_heuristic(ModelSelectionHeuristic::WeightedProbability)
        .build()
        .unwrap();
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
                    GenerationViewMode::StepByStepTimed(delay) => {
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
    data_grid: &GridData<Cartesian2D, Option<ModelInstance>, CartesianGrid<Cartesian2D>>,
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
