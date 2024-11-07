#![warn(missing_docs)]

//! This library encapsulates (and re-exports) the "ghx_proc_gen" library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse, for a Bevy usage.
//! Also provide grid utilities to manipulate & debug 2d & 3d grid data with Bevy.

/// Utilities & debug tools/plugins for using the ghx_proc_gen generator
pub mod gen;

pub use bevy_ghx_grid;
pub use ghx_proc_gen as proc_gen;

use ghx_proc_gen::ghx_grid::cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid};

#[cfg(feature = "picking")]
pub use bevy_mod_picking;

#[cfg(feature = "egui-edit")]
pub use bevy_egui;

use bevy::{ecs::bundle::Bundle, prelude::SpatialBundle};
use gen::assets::{AssetSpawner, AssetsBundleSpawner, ComponentSpawner};
use proc_gen::generator::Generator;

/// Utility [`Bundle`] to have everything necessary for generating a grid and spawning assets.
///
/// If using [`gen::simple_plugin::ProcGenSimplePlugin`] or [`gen::debug_plugin::ProcGenDebugPlugin`], this is the main `Bundle` to use.
#[derive(Bundle)]
pub struct GeneratorBundle<C: CartesianCoordinates, A: AssetsBundleSpawner, T: ComponentSpawner> {
    /// For positional rendering of the grid
    pub spatial: SpatialBundle,
    /// Grid definition (should be the same grid as in the generator)
    pub grid: CartesianGrid<C>,
    /// Generator
    pub generator: Generator<C, CartesianGrid<C>>,
    /// Assets information used when spawning nodes
    pub asset_spawner: AssetSpawner<A, T>,
}
