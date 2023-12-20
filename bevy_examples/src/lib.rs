use std::collections::HashMap;

use anim::SpawningScaleAnimation;
use bevy::{
    asset::{Asset, Handle},
    ecs::{bundle::Bundle, component::Component, entity::Entity, system::Resource},
    math::Vec3,
    time::Timer,
};
use bevy_ghx_proc_gen::{
    grid::SharableDirectionSet,
    proc_gen::generator::{observer::QueuedObserver, Generator},
};

pub mod anim;
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
    pub observer: QueuedObserver,
    /// Size of a node in world units
    pub node_scale: Vec3,
    /// Grid entity
    pub grid_entity: Entity,
    /// Scale of the spawned assets (before any animation).
    pub assets_initial_scale: Vec3,
    pub spawn_animation: Option<SpawningScaleAnimation>,
    pub bundle_spawner: fn(asset: Handle<A>, translation: Vec3, scale: Vec3, rot_rad: f32) -> B,
}

/// Timer to track the generation steps when using [`GenerationViewMode::StepByStep`]
#[derive(Resource)]
pub struct GenerationTimer(pub Timer);

/// Node spawned by the generator
#[derive(Component)]
pub struct SpawnedNode;
