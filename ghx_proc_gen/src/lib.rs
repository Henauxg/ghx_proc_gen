#![warn(missing_docs)]

//! A library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse.
//! Also provide grid utilities to manipulate 23&3d grid data.

/// Model synthesis/Wave function Collapse generator
pub mod generator;
/// Grid utilities
pub mod grid;

/// Error returned by a [`generator::Generator`] when a generation fails
#[derive(thiserror::Error, Debug)]
#[error("Failed to generate, contradiction at node with index {}", node_index)]
pub struct GenerationError {
    /// Node index at which the contradiction occurred
    pub node_index: usize,
}

/// Error returned by a [`generator::rules::RulesBuilder`] when correct [`generator::rules::Rules`] cannot be built
#[derive(thiserror::Error, Debug)]
pub enum RulesError {
    /// Rules cannot be built without models or sockets
    #[error("Empty models or sockets collection")]
    NoModelsOrSockets,
}

#[cfg(test)]
mod tests {
    use crate::{
        generator::{
            builder::GeneratorBuilder,
            rules::RulesBuilder,
            socket::{SocketCollection, SocketsCartesian2D},
        },
        grid::GridDefinition,
    };

    #[test]
    fn proc_gen_quickstart() {
        // A SocketCollection is what we use to create sockets and define their connections
        let mut sockets = SocketCollection::new();
        let (white, black) = (sockets.create(), sockets.create());

        // With the following, a white socket can connect to a black socket and vice-versa
        sockets.add_connection(white, vec![black]);

        // We define 2 very simple models, a white tile model with the `white` socket on each side and a black tile model with the `black` socket on each side
        let models = vec![
            SocketsCartesian2D::Mono(white).new_model(),
            SocketsCartesian2D::Mono(black).new_model(),
        ];

        // We give those to a RulesBuilder and get our Rules
        let rules = RulesBuilder::new_cartesian_2d(models, sockets)
            .build()
            .unwrap();

        // Like a chess board, let's do an 8x8 2d grid
        let grid = GridDefinition::new_cartesian_2d(8, 8, false, false);

        // There many more parameters you can tweak on a Generator before building it, explore the API.
        let mut generator = GeneratorBuilder::new()
            .with_rules(rules)
            .with_grid(grid)
            .build();

        // Here we directly generate the whole grid, and ask for the result to be returned.
        // The generation could also be done iteratively via `generator.select_and_propagate()`, or the results could be obtained through an `Observer`
        let checker_pattern = generator.generate_collected().unwrap();

        let icons = vec!["◻️ ", "⬛"];
        for y in 0..checker_pattern.grid().size_y() {
            for x in 0..checker_pattern.grid().size_x() {
                print!("{}", icons[checker_pattern.get_2d(x, y).model_index]);
            }
            println!();
        }
    }
}
