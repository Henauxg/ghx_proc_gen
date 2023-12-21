use std::collections::HashMap;

use anim::SpawningScaleAnimation;
use bevy::{
    asset::{Asset, Handle},
    ecs::{bundle::Bundle, component::Component, entity::Entity, system::Resource},
    math::Vec3,
    time::Timer,
};
use bevy_ghx_proc_gen::{grid::SharableDirectionSet, proc_gen::generator::Generator};

pub mod anim;
pub mod camera;
pub mod plugin;
pub mod utils;

/// Controls how the generation occurs.
#[derive(PartialEq, Eq)]
pub enum GenerationViewMode {
    /// Generates step by step and waits at least the specified amount (in milliseconds) between each step.
    StepByStep(u64),
    /// Generates step by step and waits for a user input between each step.
    StepByStepPaused,
    /// Generates it all at once at the start
    Final,
}

#[derive(Resource)]
pub struct Generation<T: SharableDirectionSet, A: Asset, B: Bundle> {
    pub models_assets: HashMap<usize, Handle<A>>,
    pub gen: Generator<T>,
    /// Size of a node in world units
    pub node_scale: Vec3,
    /// Grid entity
    pub grid_entity: Entity,
    /// Scale of the spawned assets (before any animation, if any).
    pub assets_scale: Vec3,
    /// Called to spawn the appropriate [`Bundle`] for a node
    pub bundle_spawner: fn(asset: Handle<A>, translation: Vec3, scale: Vec3, rot_rad: f32) -> B,
    /// Animation used by all spawned assets
    pub spawn_animation: Option<SpawningScaleAnimation>,
}

#[derive(Resource)]
pub struct GenerationControl {
    status: GenerationControlStatus,
    /// Whether or not the spawning systems should skip over when nodes without assets are generated.
    pub skip_void_nodes: bool,
    /// Whether or not the generation should pause when successful
    pub pause_when_done: bool,
    /// Whether or not the generation should pause when it fails
    pub pause_on_error: bool,
}

impl GenerationControl {
    pub fn new(skip_void_nodes: bool, pause_when_done: bool, pause_on_error: bool) -> Self {
        Self {
            status: GenerationControlStatus::Ongoing,
            skip_void_nodes,
            pause_on_error,
            pause_when_done,
        }
    }
}

#[derive(Resource, Eq, PartialEq, Debug)]
pub enum GenerationControlStatus {
    PausedNeedClear,
    Ongoing,
}

impl<T: SharableDirectionSet, A: Asset, B: Bundle> Generation<T, A, B> {
    pub fn new(
        models_assets: HashMap<usize, Handle<A>>,
        gen: Generator<T>,
        node_scale: Vec3,
        grid_entity: Entity,
        assets_scale: Vec3,
        bundle_spawner: fn(asset: Handle<A>, translation: Vec3, scale: Vec3, rot_rad: f32) -> B,
        spawn_animation: Option<SpawningScaleAnimation>,
    ) -> Generation<T, A, B> {
        Self {
            models_assets,
            gen,
            node_scale,
            grid_entity,
            assets_scale,
            bundle_spawner,
            spawn_animation,
        }
    }
}

/// Timer to track the generation steps when using [`GenerationViewMode::StepByStep`]
#[derive(Resource)]
pub struct GenerationTimer(pub Timer);

/// Node spawned by the generator
#[derive(Component)]
pub struct SpawnedNode;
