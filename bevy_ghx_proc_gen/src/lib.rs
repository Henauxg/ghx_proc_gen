#![warn(missing_docs)]

//! This library re-exports the "ghx_proc_gen" library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse.
//! Also provide grid utilities to manipulate & debug 2d & 3d grid data with Bevy.

pub mod gen;
/// Utilities & debug tools for manipulating grids
pub mod grid;

pub use ghx_proc_gen as proc_gen;

use bevy::{asset::Asset, ecs::bundle::Bundle, prelude::SpatialBundle};
use gen::Generation;
use grid::{Grid, SharableCoordSystem};

#[derive(Bundle)]
pub struct GeneratorBundle<C: SharableCoordSystem, A: Asset, B: Bundle> {
    pub spatial: SpatialBundle,
    pub grid: Grid<C>,
    pub generation: Generation<C, A, B>,
}
