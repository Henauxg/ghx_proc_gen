pub mod generator;
pub mod grid;

/// Errors that can occur in Ghx_ProcGen
#[derive(thiserror::Error, Debug)]
pub enum ProcGenError {
    #[error("Failed to generate")]
    GenerationFailure,
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        generator::{
            builder::GeneratorBuilder,
            node::{NodeModel, NodeRotation},
            rules::Rules,
            NodeSelectionHeuristic, RngMode,
        },
        grid::Grid,
    };

    #[test]
    fn generate_test() {
        tracing_subscriber::fmt::init();
        let models = vec![
            // corner
            NodeModel::new_2d(vec![0], vec![0], vec![1], vec![1]).with_all_rotations(),
            // cross
            NodeModel::new_2d(vec![1], vec![1], vec![1], vec![1]).with_no_rotations(),
            // empty
            NodeModel::new_2d(vec![0], vec![0], vec![0], vec![0]).with_no_rotations(),
            // line
            NodeModel::new_2d(vec![0], vec![1], vec![0], vec![1])
                .with_rotation(NodeRotation::Rot90),
            // T intersection
            NodeModel::new_2d(vec![0], vec![1], vec![1], vec![1]).with_all_rotations(),
        ];
        let rules = Rules::new_cartesian_2d(models);
        let grid = Grid::new_cartesian_2d(5, 5, false);
        let mut generator = GeneratorBuilder::new()
            .with_rules(rules)
            .with_grid(grid)
            .with_max_retry_count(10)
            .build();
        let output = generator.generate().unwrap();
    }

    #[test]
    fn generate_test_ascii() {
        tracing_subscriber::fmt::init();
        let models = vec![
            NodeModel::new_2d(vec![3], vec![3], vec![3], vec![3]), // Mountain
            NodeModel::new_2d(vec![2, 3], vec![2, 3], vec![2, 3], vec![2, 3]).with_weight(0.5), // Forest1
            NodeModel::new_2d(vec![2, 3], vec![2, 3], vec![2, 3], vec![2, 3]).with_weight(0.5), // Forest2
            NodeModel::new_2d(vec![2, 1], vec![2, 1], vec![2, 1], vec![2, 1]), // Meadows
            NodeModel::new_2d(vec![0], vec![0], vec![0], vec![0]).with_weight(1.5), // Sea
            NodeModel::new_2d(vec![0], vec![0, 1], vec![0, 1], vec![0, 1])
                .with_weight(0.25)
                .with_all_rotations(), // Beach
        ];
        let rules = Rc::new(Rules::new_cartesian_2d(models));
        let repeat_count = 1;
        for _ in 0..repeat_count {
            let size_x = 11;
            let size_y = 8;
            let grid = Grid::new_cartesian_2d(size_x, size_y, false);
            let mut generator = GeneratorBuilder::new()
                .with_shared_rules(Rc::clone(&rules))
                .with_grid(grid)
                .with_max_retry_count(750)
                .with_rng(RngMode::Random)
                .with_node_heuristic(NodeSelectionHeuristic::MinimumRemainingValue)
                .build();
            let output = generator.generate().unwrap();

            for y in (0..size_y).rev() {
                for x in 0..size_x {
                    match output.get_2d(x, y).index {
                        0 => print!("🗻"),
                        1 => print!("🌲"),
                        2 => print!("🌳"),
                        3 => print!("🟩"),
                        4 => print!("🟦"), // 🌊
                        _ => print!("🟨"),
                    }
                }
                println!();
            }
        }
    }
}
