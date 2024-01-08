use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    ecs::component::Component,
    math::{Vec2, Vec3},
    pbr::MaterialPlugin,
};
use ghx_proc_gen::grid::{direction::CoordinateSystem, GridDefinition, GridPosition};

use self::{
    lines::LineMaterial,
    markers::{draw_debug_markers_2d, draw_debug_markers_3d, update_debug_markers, MarkerEvent},
    view::{draw_debug_grids_2d, spawn_debug_grids_3d, update_debug_grid_mesh_visibility_3d},
};

pub mod lines;
pub mod markers;
pub mod view;

pub trait SharableCoordSystem: CoordinateSystem + Clone + Sync + Send + 'static {}
impl<T: CoordinateSystem + Clone + Sync + Send + 'static> SharableCoordSystem for T {}

#[derive(Component)]
pub struct Grid<D: SharableCoordSystem> {
    pub def: GridDefinition<D>,
}

pub struct GridDebugPlugin<D: SharableCoordSystem> {
    typestate: PhantomData<D>,
}

impl<T: SharableCoordSystem> GridDebugPlugin<T> {
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

impl<D: SharableCoordSystem> Plugin for GridDebugPlugin<D> {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LineMaterial>::default());
        app.add_systems(
            Update,
            (
                spawn_debug_grids_3d::<D>,
                update_debug_grid_mesh_visibility_3d,
                draw_debug_grids_2d::<D>,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                update_debug_markers::<D>,
                draw_debug_markers_3d,
                draw_debug_markers_2d,
            ),
        )
        .add_event::<MarkerEvent>();
    }
}

#[inline]
pub fn get_translation_from_grid_pos_3d(grid_pos: &GridPosition, node_size: &Vec3) -> Vec3 {
    Vec3 {
        x: (grid_pos.x as f32 + 0.5) * node_size.x,
        y: (grid_pos.y as f32 + 0.5) * node_size.y,
        z: (grid_pos.z as f32 + 0.5) * node_size.z,
    }
}

#[inline]
pub fn get_translation_from_grid_pos_2d(grid_pos: &GridPosition, node_size: &Vec2) -> Vec2 {
    Vec2 {
        x: (grid_pos.x as f32 + 0.5) * node_size.x,
        y: (grid_pos.y as f32 + 0.5) * node_size.y,
    }
}
