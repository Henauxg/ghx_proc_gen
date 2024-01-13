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
