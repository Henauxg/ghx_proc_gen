use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    ecs::{bundle::Bundle, system::Query},
    math::{Vec2, Vec3},
    pbr::MaterialPlugin,
};
use ghx_proc_gen::grid::{direction::CoordinateSystem, GridPosition};

use self::{
    lines::LineMaterial,
    markers::{
        draw_debug_markers_2d, draw_debug_markers_3d, update_debug_markers, MarkerDespawnEvent,
    },
    view::{
        draw_debug_grids_2d, draw_debug_grids_3d, DebugGridView, DebugGridViewConfig2d,
        DebugGridViewConfig3d,
    },
};

/// Shaders and materials for 3d line rendering
pub mod lines;
/// Defines markers drawn as [bevy::prelude::Gizmos], useful for debugging & visualization
pub mod markers;
/// Components and systems to visualize 2d & 3d grids
pub mod view;

/// Bevy plugin used to visualize [`ghx_proc_gen::grid::GridDefinition`] and additional debug markers created with [`markers::MarkerDespawnEvent`].
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
        app.add_systems(Update, (draw_debug_grids_3d::<C>, draw_debug_grids_2d::<C>))
            .add_systems(
                PostUpdate,
                (
                    update_debug_markers,
                    draw_debug_markers_3d,
                    draw_debug_markers_2d,
                ),
            )
            .add_event::<MarkerDespawnEvent>();
    }
}

/// Add this bundle to a [`bevy::prelude::Entity`] with a [`ghx_proc_gen::grid::GridDefinition`] if you are using a 3d camera ([`bevy::prelude::Camera3d`]).
#[derive(Bundle)]
pub struct DebugGridView3dBundle {
    /// 3d-specific configuration of the debug view
    pub config: DebugGridViewConfig3d,
    /// Debug view of the grid
    pub view: DebugGridView,
}
impl Default for DebugGridView3dBundle {
    fn default() -> Self {
        Self {
            config: Default::default(),
            view: Default::default(),
        }
    }
}

/// Add this bundle to a [`bevy::prelude::Entity`] with a [`ghx_proc_gen::grid::GridDefinition`] if you are using a 2d camera ([`bevy::prelude::Camera2d`]).
#[derive(Bundle)]
pub struct DebugGridView2dBundle {
    /// 2d-specific configuration of the debug view
    pub config: DebugGridViewConfig2d,
    /// Debug view of the grid
    pub view: DebugGridView,
}
impl Default for DebugGridView2dBundle {
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

/// Toggles the debug grids visibility
///
/// ### Example
///
/// Toggles On/Off debug grids by pressing F1
///
/// ```rust,ignore
///  app.add_systems(
///    Update,
///    toggle_debug_grids_visibilities.run_if(input_just_pressed(KeyCode::F1)),
///  );
/// ```
pub fn toggle_debug_grids_visibilities(mut grid_views: Query<&mut DebugGridView>) {
    for mut view in grid_views.iter_mut() {
        view.display_grid = !view.display_grid;
    }
}
