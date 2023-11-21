pub mod generator;
pub mod grid;

/// Errors that can occur in Ghx_ProcGen
#[derive(thiserror::Error, Debug)]
pub enum ProcGenError {
    #[error("Failed to generate")]
    GenerationFailure(),
}

#[cfg(test)]
mod tests {
    use crate::{
        generator::{node::NodeModel, rules::Rules, Generator},
        grid::{Grid, CARTESIAN_2D},
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
        let rules = Rules::new(models, CARTESIAN_2D);
        let grid = Grid::new_cartesian_2d(8, 8, false);
        let mut generator = Generator::builder()
            .with_rules(rules)
            .with_grid(grid)
            .build();
        generator.generate().unwrap();
    }
}
