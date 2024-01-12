use std::{collections::HashMap, sync::Arc};

use bevy::{
    asset::{Asset, Handle},
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query, Res, Resource},
    },
    hierarchy::BuildChildren,
    math::{Quat, Vec3},
    prelude::SpatialBundle,
    render::texture::Image,
    scene::{Scene, SceneBundle},
    sprite::SpriteBundle,
    transform::components::Transform,
    utils::default,
};
use ghx_proc_gen::{
    generator::{
        model::{ModelIndex, ModelInstance},
        observer::QueuedObserver,
        Generator,
    },
    grid::direction::GridDelta,
};

use crate::grid::{Grid, SharableCoordSystem};

pub mod debug_plugin;
pub mod simple_plugin;

/// Marker for nodes spawned by the generator
#[derive(Component)]
pub struct SpawnedNode;

/// Represents an asset for a model
pub struct ModelAsset<A: Asset> {
    pub handle: Handle<A>,
    pub offset: GridDelta,
}

impl<A: Asset> ModelAsset<A> {
    pub fn handle(&self) -> &Handle<A> {
        &self.handle
    }
    pub fn offset(&self) -> &GridDelta {
        &self.offset
    }
}

pub type RulesModelsAssets<A: Asset> = HashMap<ModelIndex, Vec<ModelAsset<A>>>;

pub type BundleSpawner<A: Asset, B: Bundle> =
    fn(asset: Handle<A>, translation: Vec3, scale: Vec3, rot_rad: f32) -> B;

// Since we do only 1 generation at a time, we put it all in a resource
#[derive(Component)]
pub struct Generation<C: SharableCoordSystem, A: Asset, B: Bundle> {
    pub gen: Generator<C>,
    pub models_assets: Arc<RulesModelsAssets<A>>,
    pub observer: QueuedObserver,
    /// Size of a node in world units
    pub node_size: Vec3,
    /// Scale of the assets when spawned
    pub assets_spawn_scale: Vec3,
    /// Called to spawn the appropriate [`Bundle`] for a node
    pub asset_bundle_spawner: BundleSpawner<A, B>,
    /// Whether to offset the z coordinate of spawned nodes from the y coordinate (used for 2d ordering of sprites)
    pub z_offset_from_y: bool,
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> Generation<C, A, B> {
    pub fn new(
        mut gen: Generator<C>,
        models_assets: Arc<RulesModelsAssets<A>>,
        node_size: Vec3,
        assets_spawn_scale: Vec3,
        asset_bundle_spawner: BundleSpawner<A, B>,
    ) -> Generation<C, A, B> {
        let observer = QueuedObserver::new(&mut gen);
        Self {
            gen,
            observer,
            node_size,
            models_assets,
            assets_spawn_scale,
            asset_bundle_spawner,
            z_offset_from_y: false,
        }
    }
}

pub fn insert_default_bundle_to_spawned_nodes<B: Bundle + Default>(
    mut commands: Commands,
    spawned_nodes: Query<Entity, Added<SpawnedNode>>,
) {
    for node in spawned_nodes.iter() {
        commands.entity(node).insert(B::default());
    }
}

pub fn insert_bundle_from_resource_to_spawned_nodes<B: Bundle + Resource + Clone>(
    mut commands: Commands,
    bundle_to_clone: Res<B>,
    spawned_nodes: Query<Entity, Added<SpawnedNode>>,
) {
    for node in spawned_nodes.iter() {
        commands.entity(node).insert(bundle_to_clone.clone());
    }
}

/// Uses the z+ axis as the rotation axis
pub fn sprite_node_spawner(
    texture: Handle<Image>,
    translation: Vec3,
    scale: Vec3,
    rot_rad: f32,
) -> SpriteBundle {
    SpriteBundle {
        texture,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_z(rot_rad)),
        ..default()
    }
}

/// Uses the y+ axis as the rotation axis
pub fn scene_node_spawner(
    scene: Handle<Scene>,
    translation: Vec3,
    scale: Vec3,
    rot_rad: f32,
) -> SceneBundle {
    SceneBundle {
        scene,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rot_rad)),
        ..default()
    }
}

pub fn spawn_node<C: SharableCoordSystem, A: Asset, B: Bundle>(
    commands: &mut Commands,
    gen_entity: Entity,
    generation: &Generation<C, A, B>,
    instance: &ModelInstance,
    node_index: usize,
) {
    let empty = vec![];
    let node_assets = generation
        .models_assets
        .get(&instance.model_index)
        .unwrap_or(&empty);
    if node_assets.is_empty() {
        return;
    }

    let pos = generation.gen.grid().get_position(node_index);
    for node_asset in node_assets {
        let offset = node_asset.offset();
        // +0.5*scale to center the node because its center is at its origin
        let mut translation = Vec3::new(
            generation.node_size.x * (pos.x as f32 + offset.dx as f32 + 0.5),
            generation.node_size.y * (pos.y as f32 + offset.dy as f32 + 0.5),
            generation.node_size.z * (pos.z as f32 + offset.dz as f32 + 0.5),
        );

        if generation.z_offset_from_y {
            translation.z += generation.node_size.z
                * (1. - pos.y as f32 / generation.gen.grid().size_y() as f32);
        }

        let node_entity = commands
            .spawn((
                (generation.asset_bundle_spawner)(
                    node_asset.handle.clone(),
                    translation,
                    generation.assets_spawn_scale,
                    f32::to_radians(instance.rotation.value() as f32),
                ),
                SpawnedNode,
            ))
            .id();
        commands.entity(gen_entity).add_child(node_entity);
    }
}
