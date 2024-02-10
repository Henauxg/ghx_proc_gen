#![warn(missing_docs)]

//! This library encapsulates (and re-exports) the "ghx_proc_gen" library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse, for a Bevy usage.
//! Also provide grid utilities to manipulate & debug 2d & 3d grid data with Bevy.

/// Utilities & debug tools/plugins for using the ghx_proc_gen generator
pub mod gen;
/// Utilities & debug tools/plugins for manipulating grids
#[cfg(feature = "grid-debug-plugin")]
pub mod grid;

pub use ghx_proc_gen as proc_gen;

#[cfg(feature = "picking")]
pub use bevy_mod_picking;

#[cfg(feature = "egui-edit")]
pub use bevy_egui;

use bevy::{ecs::bundle::Bundle, prelude::SpatialBundle};
use gen::assets::{AssetSpawner, AssetsBundleSpawner, ComponentSpawner};
use proc_gen::{
    generator::Generator,
    grid::{direction::CoordinateSystem, GridDefinition},
};

/// Utility [`Bundle`] to have everything necessary for generating a grid and spawning assets.
///
/// If using [`gen::simple_plugin::ProcGenSimplePlugin`] or [`gen::debug_plugin::ProcGenDebugPlugin`], this is the main `Bundle` to use.
#[derive(Bundle)]
pub struct GeneratorBundle<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> {
    /// For positional rendering of the grid
    pub spatial: SpatialBundle,
    /// Grid definition (Should be the same [`proc_gen::grid::GridDefinition`] as in the generator)
    pub grid: GridDefinition<C>,
    /// Generator
    pub generator: Generator<C>,
    /// Assets information used when spawning nodes
    pub asset_spawner: AssetSpawner<A, T>,
}
