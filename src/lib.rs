pub mod generator;
pub mod grid;

/// Errors that can occur in Ghx_ProcGen
#[derive(thiserror::Error, Debug)]
pub enum ProcGenError {
    #[error("Failed to generate")]
    GenerationFailure,
    #[error("Configuration failure")]
    ConfigurationFailure,
}

#[cfg(test)]
mod tests {
    use crate::{
        generator::{builder::GeneratorBuilder, node::NodeModel, rules::Rules},
        grid::Grid,
    };

    #[test]
    fn generate_test() {
        let models = vec![
            // corner
            NodeModel::new_2d(vec![0], vec![0], vec![1], vec![1]),
            // cross
            NodeModel::new_2d(vec![1], vec![1], vec![1], vec![1]),
            // empty
            NodeModel::new_2d(vec![0], vec![0], vec![0], vec![0]),
            // line
            NodeModel::new_2d(vec![0], vec![1], vec![0], vec![1]),
            // T intersection
            NodeModel::new_2d(vec![0], vec![1], vec![1], vec![1]),
        ];
        let rules = Rules::new_cartesian_2d(models);
        let grid = Grid::new_cartesian_2d(8, 8, false);
        let mut generator = GeneratorBuilder::new()
            .with_rules(rules)
            .with_grid(grid)
            .build();
        generator.generate().unwrap();
    }
}
