use std::{collections::HashMap, sync::Arc};

use bevy::{
    asset::Handle,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query, Res, Resource},
    },
    hierarchy::BuildChildren,
    math::{Quat, Vec3},
    pbr::{Material, MaterialMeshBundle, PbrBundle, StandardMaterial},
    render::{mesh::Mesh, texture::Image},
    scene::{Scene, SceneBundle},
    sprite::SpriteBundle,
    transform::components::Transform,
    utils::default,
};
use ghx_proc_gen::{
    generator::{
        model::{ModelIndex, ModelInstance},
        Generator,
    },
    grid::direction::GridDelta,
};

use crate::grid::SharableCoordSystem;

/// Debug plugin to run the generation & spawn assets automatically with different visualization options
pub mod debug_plugin;
/// Simple plugin to run the generation & spawn assets automatically
pub mod simple_plugin;

/// Marker for nodes spawned by the generator
#[derive(Component)]
pub struct SpawnedNode;

pub trait AssetHandles: Clone + Sync + Send + 'static {}
impl<T: Clone + Sync + Send + 'static> AssetHandles for T {}

/// Represents an asset for a model
#[derive(Clone)]
pub struct ModelAsset<A: AssetHandles> {
    /// Handle(s) to the asset(s)
    pub handles: A,
    /// Offset from the generated grid node. The asset will be instantiated at `generated_node_grid_pos + offset`
    pub offset: GridDelta,
}

/// Defines a map which links a `Model` via its [`ModelIndex`] to his spawnable assets
pub struct RulesModelsAssets<A: AssetHandles> {
    pub map: HashMap<ModelIndex, Vec<ModelAsset<A>>>,
}

impl<A: AssetHandles> RulesModelsAssets<A> {
    pub fn new() -> Self {
        Self { map: default() }
    }

    pub fn add_asset(&mut self, index: ModelIndex, asset: A) {
        let model_asset = ModelAsset {
            handles: asset,
            offset: default(),
        };
        self.add(index, model_asset);
    }

    pub fn add(&mut self, index: ModelIndex, model_asset: ModelAsset<A>) {
        match self.map.get_mut(&index) {
            Some(assets) => {
                assets.push(model_asset);
            }
            None => {
                self.map.insert(index, vec![model_asset]);
            }
        }
    }
}

/// Type alias. Defines a function which from an [`AssetHandles`], a position, a scale and a rotation (in radians) returns a spawnable [`Bundle`]
pub type BundleSpawner<A, B> = fn(assets: A, translation: Vec3, scale: Vec3, rot_rad: f32) -> B;

/// Encapsulates a [`Generator`] and other information needed to correclty spawn assets
#[derive(Component)]
pub struct Generation<C: SharableCoordSystem, A: AssetHandles, B: Bundle> {
    /// The generator that will produce the [`ModelInstance`]
    pub gen: Generator<C>,
    /// Link a `Model` via its [`ModelIndex`] to his spawnable assets (can be shared by multiple [`Generation`])
    pub assets: Arc<RulesModelsAssets<A>>,
    /// Size of a node in world units
    pub node_size: Vec3,
    /// Scale of the assets when spawned
    pub assets_spawn_scale: Vec3,
    /// Called to spawn the appropriate [`Bundle`] for a node
    pub asset_bundle_spawner: BundleSpawner<A, B>,
    /// Whether to offset the z coordinate of spawned nodes from the y coordinate (used for 2d ordering of sprites)
    pub z_offset_from_y: bool,
}

impl<C: SharableCoordSystem, A: AssetHandles, B: Bundle> Generation<C, A, B> {
    /// Constructor for a `Generation`, `z_offset_from_y` defaults to `false`
    pub fn new(
        gen: Generator<C>,
        models_assets: RulesModelsAssets<A>,
        node_size: Vec3,
        assets_spawn_scale: Vec3,
        asset_bundle_spawner: BundleSpawner<A, B>,
    ) -> Generation<C, A, B> {
        Self {
            gen,
            node_size,
            assets: Arc::new(models_assets),
            assets_spawn_scale,
            asset_bundle_spawner,
            z_offset_from_y: false,
        }
    }
}

/// Utility system. Adds a [`Bundle`] (or a [`Component`]) to every [`Entity`] that has [`SpawnedNode`] Component (this is the case of nodes spawned by the `spawn_node` system). The `Bundle` will have its default value.
///
/// ### Example
///
/// Add a `MyAnimation` Component with its default value to every newly spawned node Entity
/// ```ignore
/// #[derive(Component, Default)]
/// pub struct MyAnimation {
///     duration_sec: f32,
///     final_scale: Vec3,
/// }
/// impl Default for MyAnimation {
///     fn default() -> Self {
///         Self {
///             duration_sec: 5.,
///             final_scale: Vec3::splat(2.0),
///         }
///     }
/// }
/// // ... In the `App` initialization code:
/// app.add_systems(
///     Update,
///     insert_default_bundle_to_spawned_nodes::<MyAnimation>
/// );
/// ```
pub fn insert_default_bundle_to_spawned_nodes<B: Bundle + Default>(
    mut commands: Commands,
    spawned_nodes: Query<Entity, Added<SpawnedNode>>,
) {
    for node in spawned_nodes.iter() {
        commands.entity(node).insert(B::default());
    }
}

/// Utility system. Adds a [`Bundle`] (or a [`Component`]) to every [`Entity`] that has [`SpawnedNode`] Component (this is the case of nodes spawned by the `spawn_node` system). The `Bundle` will be cloned from a `Resource`
///
/// ### Example
///
/// Add a `MyAnimation` Component cloned from a `Resource` to every newly spawned node Entity
/// ```ignore
/// #[derive(Component, Resource)]
/// pub struct MyAnimation {
///     duration_sec: f32,
///     final_scale: Vec3,
/// }
/// app.insert_resource(MyAnimation {
///     duration_sec: 0.8,
///     final_scale: Vec3::ONE,
/// });
/// app.add_systems(
///     Update,
///     insert_bundle_from_resource_to_spawned_nodes::<MyAnimation>
/// );
/// ```
pub fn insert_bundle_from_resource_to_spawned_nodes<B: Bundle + Resource + Clone>(
    mut commands: Commands,
    bundle_to_clone: Res<B>,
    spawned_nodes: Query<Entity, Added<SpawnedNode>>,
) {
    for node in spawned_nodes.iter() {
        commands.entity(node).insert(bundle_to_clone.clone());
    }
}

/// Utility [`BundleSpawner`]
///
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

/// Utility [`BundleSpawner`]
///
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

#[derive(Clone)]
pub struct MaterialMesh<M: Material> {
    pub mesh: Handle<Mesh>,
    pub mat: Handle<M>,
}

#[derive(Clone)]
pub struct PbrMesh {
    pub mesh: Handle<Mesh>,
    pub mat: Handle<StandardMaterial>,
}

pub fn material_mesh_node_spawner<M: Material>(
    asset: MaterialMesh<M>,
    translation: Vec3,
    scale: Vec3,
    rot_rad: f32,
) -> MaterialMeshBundle<M> {
    MaterialMeshBundle {
        mesh: asset.mesh,
        material: asset.mat,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rot_rad)),
        ..default()
    }
}

pub fn pbr_node_spawner(asset: PbrMesh, translation: Vec3, scale: Vec3, rot_rad: f32) -> PbrBundle {
    PbrBundle {
        mesh: asset.mesh,
        material: asset.mat,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rot_rad)),
        ..default()
    }
}

/// Utility system to spawn grid nodes. Can work for multiple asset types.
///
/// Used by [`ProcGenSimplePlugin`] and [`ProcGenDebugPlugin`] to spawn assets automatically.
///
/// ### Examples
///
/// Spawn 3d models (gltf) assets with a `Cartesian3D` grid
/// ```ignore
/// spawn_node::<Cartesian3D, Scene, SceneBundle>(...);
/// ```
/// Spawn 2d sprites (png, ...) assets with a `Cartesian3D` grid
/// ```ignore
/// spawn_node::<Cartesian3D, Image, SpriteBundle>(...);
/// ```
pub fn spawn_node<C: SharableCoordSystem, A: AssetHandles, B: Bundle>(
    commands: &mut Commands,
    gen_entity: Entity,
    generation: &Generation<C, A, B>,
    instance: &ModelInstance,
    node_index: usize,
) {
    let empty = vec![];
    let node_assets = generation
        .assets
        .map
        .get(&instance.model_index)
        .unwrap_or(&empty);
    if node_assets.is_empty() {
        return;
    }

    let pos = generation.gen.grid().get_position(node_index);
    for node_asset in node_assets {
        let offset = &node_asset.offset;
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
                    node_asset.handles.clone(),
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
