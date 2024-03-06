#![warn(missing_docs)]

//! A library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse.
//! Also provide grid utilities to manipulate 2d & 3d grid data.

use generator::model::{ModelIndex, ModelRotation, ModelVariantIndex};
use ghx_grid::grid::GridIndex;

/// Model synthesis/Wave function Collapse generator
pub mod generator;

/// Our grid elements are called Nodes
pub type NodeIndex = GridIndex;

/// Error returned by a [`generator::Generator`] when a generation fails
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("Failed to generate, contradiction at node with index {}", node_index)]
pub struct GeneratorError {
    /// Node index at which the contradiction occurred
    pub node_index: NodeIndex,
}

/// Error returned by a [`generator::rules::RulesBuilder`] when correct [`generator::rules::Rules`] cannot be built
#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum RulesBuilderError {
    /// Rules cannot be built without models or sockets
    #[error("Empty models or sockets collection")]
    NoModelsOrSockets,
}

/// Error returned by a [`generator::Generator`] when a node set operation fails
#[derive(thiserror::Error, Debug, Clone)]
pub enum NodeSetError {
    /// An invalid [`ModelVariantIndex`] was given
    #[error("Invalid model variant index `{0}`, does not exist in the rules")]
    InvalidModelIndex(ModelVariantIndex),
    /// An invalid [`generator::rules::ModelVariantRef`] was given
    #[error("Invalid model variant reference: model index `{0}` with rotation `{1:?}`, does not exist in the rules")]
    InvalidModelRef(ModelIndex, ModelRotation),
    /// An invalid node index was given
    #[error("Invalid node index `{0}`, does not exist in the grid")]
    InvalidNodeIndex(NodeIndex),
    /// An operation requested to set a model on a node that does not allow it
    #[error("Model variant `{0}` not allowed by the Rules on node {1}")]
    IllegalModel(ModelVariantIndex, NodeIndex),
    /// Wraps a [`GeneratorError`]
    #[error("Generation error: {0}")]
    GenerationError(#[from] GeneratorError),
}

/// Errors returned by a [`generator::builder::GeneratorBuilder`]
#[derive(thiserror::Error, Debug, Clone)]
pub enum GeneratorBuilderError {
    /// Error returned by a [`generator::builder::GeneratorBuilder`] when a node set operation fails
    #[error("Initial node set error: {0}")]
    InitialNodeSetError(#[from] NodeSetError),
    /// Error returned by a [`generator::builder::GeneratorBuilder`] when a given grid does not match the size of the builder's grid.
    #[error("Given grid size {0:?} does not match the expected size {1:?}")]
    InvalidGridSize((u32, u32, u32), (u32, u32, u32)),
}
