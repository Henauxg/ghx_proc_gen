pub mod generator;
pub mod grid;

/// Errors that can occur in Ghx_ProcGen
#[derive(thiserror::Error, Debug)]
pub enum ProcGenError {
    #[error("Failed to generate")]
    GenerationFailure,
    #[error("Rules are invalid")]
    InvalidRules,
}

#[cfg(test)]
mod tests {

    use crate::{
        generator::{
            builder::GeneratorBuilder,
            node::{NodeRotation, SocketId, SocketsCartesian2D},
            rules::Rules,
        },
        grid::GridDefinition,
    };

    const VOID: SocketId = 0;
    const PIPE: SocketId = 1;

    #[test]
    fn generate_test() {
        let models = vec![
            // corner
            SocketsCartesian2D::Simple(VOID, VOID, PIPE, PIPE)
                .new_model()
                .with_all_rotations(),
            // cross
            SocketsCartesian2D::Simple(PIPE, PIPE, PIPE, PIPE)
                .new_model()
                .with_no_rotations(),
            // empty
            SocketsCartesian2D::Simple(VOID, VOID, VOID, VOID)
                .new_model()
                .with_no_rotations(),
            // line
            SocketsCartesian2D::Simple(VOID, PIPE, VOID, PIPE)
                .new_model()
                .with_rotation(NodeRotation::Rot90),
            // T intersection
            SocketsCartesian2D::Simple(VOID, PIPE, PIPE, PIPE)
                .new_model()
                .with_all_rotations(),
        ];
        let sockets_connections = vec![(VOID, vec![VOID]), (PIPE, vec![PIPE])];
        let rules = Rules::new_cartesian_2d(models, sockets_connections).unwrap();
        let grid = GridDefinition::new_cartesian_2d(5, 5, false);
        let mut generator = GeneratorBuilder::new()
            .with_rules(rules)
            .with_grid(grid)
            .with_max_retry_count(10)
            .build();
        let _output = generator.generate().unwrap();
    }
}
