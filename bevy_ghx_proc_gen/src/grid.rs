use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PostUpdate, Update},
    ecs::component::Component,
    math::{Vec2, Vec3},
    pbr::MaterialPlugin,
    render::color::Color,
};
use ghx_proc_gen::grid::{direction::DirectionSet, GridDefinition, GridPosition};

use self::{
    lines::LineMaterial,
    markers::{draw_debug_markers_2d, draw_debug_markers_3d, update_debug_markers, MarkerEvent},
    view::{draw_debug_grids_2d, spawn_debug_grids_2d, spawn_debug_grids_3d},
};

pub mod lines;
pub mod markers;
pub mod view;

#[derive(Component)]
pub struct DebugGridViewConfig3d {
    pub node_size: Vec3,
    pub color: Color,
}

#[derive(Component)]
pub struct DebugGridViewConfig2d {
    pub node_size: Vec2,
    pub color: Color,
}

pub trait SharableDirectionSet: DirectionSet + Clone + Sync + Send + 'static {}
impl<T: DirectionSet + Clone + Sync + Send + 'static> SharableDirectionSet for T {}

#[derive(Component)]
pub struct Grid<D: SharableDirectionSet> {
    pub def: GridDefinition<D>,
}

pub struct GridDebugPlugin<D: SharableDirectionSet> {
    typestate: PhantomData<D>,
}

impl<T: SharableDirectionSet> GridDebugPlugin<T> {
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

impl<D: SharableDirectionSet> Plugin for GridDebugPlugin<D> {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LineMaterial>::default());
        app.add_systems(
            Update,
            (
                spawn_debug_grids_3d::<D>,
                spawn_debug_grids_2d::<D>,
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
