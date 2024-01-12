#![warn(missing_docs)]

//! This library encapsulates (and re-exportes) the "ghx_proc_gen" library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse, for a Bevy usage.
//! Also provide grid utilities to manipulate & debug 2d & 3d grid data with Bevy.

/// Utilities & debug tools/plugins for using the ghx_proc_gen generator
pub mod gen;
/// Utilities & debug tools/plugins for manipulating grids
pub mod grid;

pub use ghx_proc_gen as proc_gen;

use bevy::{asset::Asset, ecs::bundle::Bundle, prelude::SpatialBundle};
use gen::Generation;
use grid::{Grid, SharableCoordSystem};

/// Utility [`Bundle`] to have everything necessary for generating a grid and spawning assets.
///
/// If using [`gen::simple_plugin::ProcGenSimplePlugin`] or [`gen::debug_plugin::ProcGenDebugPlugin`], this is the main `Bundle` to use.
#[derive(Bundle)]
pub struct GeneratorBundle<C: SharableCoordSystem, A: Asset, B: Bundle> {
    /// For positional rendering the grid
    pub spatial: SpatialBundle,
    /// Grid definition (Should be the same [`proc_gen::grid::GridDefinition`] as in the generator)
    pub grid: Grid<C>,
    /// Generator and assets information
    pub generation: Generation<C, A, B>,
}
