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
        generator::{node::NodeModel, Generator},
        grid::Grid,
    };

    #[test]
    fn generate_test() {
        let models = vec![
            // corner
            NodeModel::new(0, 0, 1, 1),
            // cross
            NodeModel::new(0, 0, 1, 1),
            // empty
            NodeModel::new(0, 0, 0, 0),
            // line
            NodeModel::new(0, 1, 0, 1),
            // T intersection
            NodeModel::new(0, 1, 1, 1),
        ];
        let grid = Grid::new_cartesian_2d_grid(8, 8, false);
        let mut generator = Generator::builder()
            .with_models(models)
            .with_grid(grid)
            .build();
        generator.generate().unwrap();
    }
}
