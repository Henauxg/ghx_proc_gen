use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    ecs::bundle::Bundle,
    math::{Vec2, Vec3},
    pbr::MaterialPlugin,
};
use ghx_proc_gen::grid::{direction::CoordinateSystem, GridPosition};

use self::{
    lines::LineMaterial,
    markers::{draw_debug_markers_2d, draw_debug_markers_3d, update_debug_markers, MarkerEvent},
    view::{
        draw_debug_grids_2d, spawn_debug_grids_3d, update_debug_grid_mesh_visibility_3d,
        DebugGridView, DebugGridViewConfig2d, DebugGridViewConfig3d,
    },
};

/// Shaders and materials for 3d line rendering
pub mod lines;
/// Defines markers drawn as [bevy::prelude::Gizmos], useful for debugging & visualization
pub mod markers;
/// Components and systems to visualize 2d & 3d grids
pub mod view;

/// Additional traits constraints on a [`CoordinateSystem`] to ensure that it can safely be shared between threads.
// pub trait CoordinateSystem: CoordinateSystem + Clone + Sync + Send + 'static {}
// impl<T: CoordinateSystem + Clone + Sync + Send + 'static> CoordinateSystem for T {}

/// Bevy plugin used to visualize [`ghx_proc_gen::grid::GridDefinition`] and additional debug markers created with [`markers::MarkerEvent`].
pub struct GridDebugPlugin<C: CoordinateSystem> {
    typestate: PhantomData<C>,
}

impl<T: CoordinateSystem> GridDebugPlugin<T> {
    /// Create a new GridDebugPlugin
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem> Plugin for GridDebugPlugin<C> {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LineMaterial>::default());
        app.add_systems(
            Update,
            (
                spawn_debug_grids_3d::<C>,
                update_debug_grid_mesh_visibility_3d,
                draw_debug_grids_2d::<C>,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                update_debug_markers::<C>,
                draw_debug_markers_3d,
                draw_debug_markers_2d,
            ),
        )
        .add_event::<MarkerEvent>();
    }
}

/// Add this bundle to an [`Entity`] with a [`Grid`] if you are using a 3d camera ([`bevy::prelude::Camera3d`]).
#[derive(Bundle)]
pub struct DebugGridView3d {
    /// 3d-specific configuration of the debug view
    pub config: DebugGridViewConfig3d,
    /// Debug view of the grid
    pub view: DebugGridView,
}
impl Default for DebugGridView3d {
    fn default() -> Self {
        Self {
            config: Default::default(),
            view: Default::default(),
        }
    }
}

/// Add this bundle to an [`Entity`] with a [`Grid`] if you are using a 2d camera ([`bevy::prelude::Camera2d`]).
#[derive(Bundle)]
pub struct DebugGridView2d {
    /// 2d-specific configuration of the debug view
    pub config: DebugGridViewConfig2d,
    /// Debug view of the grid
    pub view: DebugGridView,
}
impl Default for DebugGridView2d {
    fn default() -> Self {
        Self {
            config: Default::default(),
            view: Default::default(),
        }
    }
}

/// Transform a [`GridPosition`] accompanied by a `node_size`, the size of a grid node in world units, into a position as a [`Vec3`] in world units (center of the grid node).
#[inline]
pub fn get_translation_from_grid_pos_3d(grid_pos: &GridPosition, node_size: &Vec3) -> Vec3 {
    Vec3 {
        x: (grid_pos.x as f32 + 0.5) * node_size.x,
        y: (grid_pos.y as f32 + 0.5) * node_size.y,
        z: (grid_pos.z as f32 + 0.5) * node_size.z,
    }
}

/// Transform a [`GridPosition`] accompanied by a `node_size`, the size of a grid node in world units, into a position as a [`Vec2`] in world units (center of the grid node).
#[inline]
pub fn get_translation_from_grid_pos_2d(grid_pos: &GridPosition, node_size: &Vec2) -> Vec2 {
    Vec2 {
        x: (grid_pos.x as f32 + 0.5) * node_size.x,
        y: (grid_pos.y as f32 + 0.5) * node_size.y,
    }
}
