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
            node::{NodeRotation, SocketsCartesian2D},
            rules::Rules,
        },
        grid::Grid,
    };

    #[test]
    fn generate_test() {
        let models = vec![
            // corner
            SocketsCartesian2D::Simple(0, 0, 1, 1)
                .new_model()
                .with_all_rotations(),
            // cross
            SocketsCartesian2D::Simple(1, 1, 1, 1)
                .new_model()
                .with_no_rotations(),
            // empty
            SocketsCartesian2D::Simple(0, 0, 0, 0)
                .new_model()
                .with_no_rotations(),
            // line
            SocketsCartesian2D::Simple(0, 1, 0, 1)
                .new_model()
                .with_rotation(NodeRotation::Rot90),
            // T intersection
            SocketsCartesian2D::Simple(0, 1, 1, 1)
                .new_model()
                .with_all_rotations(),
        ];
        let rules = Rules::new_cartesian_2d(models);
        let grid = Grid::new_cartesian_2d(5, 5, false);
        let mut generator = GeneratorBuilder::new()
            .with_rules(rules)
            .with_grid(grid)
            .with_max_retry_count(10)
            .build();
        let _output = generator.generate().unwrap();
    }
}
