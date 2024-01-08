#![warn(missing_docs)]

//! This library re-exports the "ghx_proc_gen" library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse.
//! Also provide grid utilities to manipulate & debug 2d & 3d grid data with Bevy.

/// Utilities & debug tools for manipulating grids
pub mod grid;

pub use ghx_proc_gen as proc_gen;
