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
            rules::RulesBuilder,
        },
        grid::GridDefinition,
    };

    const VOID: SocketId = 0;
    const PIPE: SocketId = 1;

    #[test]
    fn generate_test() {
        let models = vec![
            // corner
            SocketsCartesian2D::Simple {
                x_pos: PIPE,
                x_neg: VOID,
                y_pos: VOID,
                y_neg: PIPE,
            }
            .new_model()
            .with_all_rotations(),
            // cross
            SocketsCartesian2D::Mono(PIPE).new_model(),
            // empty
            SocketsCartesian2D::Mono(VOID).new_model(),
            // line
            SocketsCartesian2D::Simple {
                x_pos: VOID,
                x_neg: VOID,
                y_pos: PIPE,
                y_neg: PIPE,
            }
            .new_model()
            .with_rotation(NodeRotation::Rot90),
            // T intersection
            SocketsCartesian2D::Simple {
                x_pos: PIPE,
                x_neg: PIPE,
                y_pos: VOID,
                y_neg: PIPE,
            }
            .new_model()
            .with_all_rotations(),
        ];
        let sockets_connections = vec![(VOID, vec![VOID]), (PIPE, vec![PIPE])];
        let rules = RulesBuilder::new_cartesian_2d(models, sockets_connections)
            .build()
            .unwrap();
        let grid = GridDefinition::new_cartesian_2d(5, 5, false);
        let mut generator = GeneratorBuilder::new()
            .with_rules(rules)
            .with_grid(grid)
            .with_max_retry_count(10)
            .build();
        let _output = generator.generate_collected().unwrap();
    }
}
