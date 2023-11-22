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
    use crate::{
        generator::{
            builder::GeneratorBuilder,
            node::{NodeModel, NodeRotation},
            rules::Rules,
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
        generator.generate().unwrap();
    }
}
