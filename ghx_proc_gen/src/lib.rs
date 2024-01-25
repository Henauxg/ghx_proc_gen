#![warn(missing_docs)]

//! A library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse.
//! Also provide grid utilities to manipulate 23&3d grid data.

use generator::model::{ModelIndex, ModelRotation, ModelVariantIndex};
use grid::NodeIndex;

/// Model synthesis/Wave function Collapse generator
pub mod generator;
/// Grid utilities
pub mod grid;

/// Error returned by a [`generator::Generator`] when a generation fails
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("Failed to generate, contradiction at node with index {}", node_index)]
pub struct GenerationError {
    /// Node index at which the contradiction occurred
    pub node_index: NodeIndex,
}

/// Error returned by a [`generator::rules::RulesBuilder`] when correct [`generator::rules::Rules`] cannot be built
#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum RulesError {
    /// Rules cannot be built without models or sockets
    #[error("Empty models or sockets collection")]
    NoModelsOrSockets,
}

/// Error returned by a [`generator::Generator`] or a [`generator::builder::GeneratorBuilder`] when a node set operation fails
#[derive(thiserror::Error, Debug, Clone)]
pub enum NodeSetError {
    #[error("Invalid model variant index `{0}`, does not exist in the rules")]
    InvalidModelIndex(ModelVariantIndex),
    #[error("Invalid model variant reference: model index `{0}` with rotation `{1:?}`, does not exist in the rules")]
    InvalidModelRef(ModelIndex, ModelRotation),
    #[error("Invalid node index `{0}`, does not exist in the grid")]
    InvalidNodeIndex(NodeIndex),
    #[error("Model variant `{0}` not allowed by the Rules on node {1}")]
    IllegalModel(ModelVariantIndex, NodeIndex),
    #[error("Generation error: {0}")]
    GenerationError(#[from] GenerationError),
}

#[derive(thiserror::Error, Debug, Clone)]
#[error("Given grid size {0:?} does not match the expected size {1:?}")]
pub struct InvalidGridSize((u32, u32, u32), (u32, u32, u32));
