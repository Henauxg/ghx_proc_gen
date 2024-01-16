use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

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
        model::{ModelIndex, ModelInstance, ModelRotation},
        Generator,
    },
    grid::direction::{CoordinateSystem, GridDelta},
};

/// Debug plugin to run the generation & spawn assets automatically with different visualization options
pub mod debug_plugin;
/// Simple plugin to run the generation & spawn assets automatically
pub mod simple_plugin;

/// Marker for nodes spawned by the generator
#[derive(Component)]
pub struct SpawnedNode;

/// Used as a custom trait for types which store things such as handles to assets
pub trait AssetHandles: Clone + Sync + Send + 'static {}
impl<T: Clone + Sync + Send + 'static> AssetHandles for T {}

/// Represents an asset for a model
#[derive(Clone)]
pub struct ModelAsset<A: AssetHandles, T: ComponentWrapper = NoComponents> {
    /// Handle(s) to the asset(s)
    pub handles: A,
    /// Offset from the generated grid node. The asset will be instantiated at `generated_node_grid_pos + offset`
    pub offset: GridDelta,
    pub components: Vec<T>,
}

/// Defines a map which links a `Model` via its [`ModelIndex`] to his spawnable assets
pub struct RulesModelsAssets<A: AssetHandles, T: ComponentWrapper = NoComponents> {
    /// Only contains a ModelIndex if there are some assets for it
    map: HashMap<ModelIndex, Vec<ModelAsset<A, T>>>,
}
impl<A: AssetHandles, T: ComponentWrapper> Deref for RulesModelsAssets<A, T> {
    type Target = HashMap<ModelIndex, Vec<ModelAsset<A, T>>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl<A: AssetHandles, T: ComponentWrapper> DerefMut for RulesModelsAssets<A, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<A: AssetHandles, T: ComponentWrapper> RulesModelsAssets<A, T> {
    /// Create a new RulesModelsAssets with an empty map
    pub fn new() -> Self {
        Self { map: default() }
    }

    /// Create a new RulesModelsAssets with an existing map
    pub fn new_from_map(map: HashMap<ModelIndex, Vec<ModelAsset<A, T>>>) -> Self {
        Self { map }
    }

    /// Adds a [`ModelAsset`] with no grid offset, to the model `index`
    pub fn add_asset(&mut self, index: ModelIndex, asset: A) {
        let model_asset = ModelAsset {
            handles: asset,
            offset: default(),
            components: Vec::new(),
        };
        self.add(index, model_asset);
    }

    /// Adds a [`ModelAsset`] to the model `index`
    pub fn add(&mut self, index: ModelIndex, model_asset: ModelAsset<A, T>) {
        match self.get_mut(&index) {
            Some(assets) => {
                assets.push(model_asset);
            }
            None => {
                self.insert(index, vec![model_asset]);
            }
        }
    }
}

/// Type alias. Defines a function which from an [`AssetHandles`], a position, a scale and a rotation, returns a spawnable [`Bundle`]
///
/// You can make your own, or use one of the simple ones provided:
/// - `sprite_node_spawner` for A: `Handle<Image>` and B: `SpriteBundle`
/// - `scene_node_spawner` for A: `Handle<Scene>` and B: `SceneBundle`
/// - `material_mesh_node_spawner` for A: `MaterialMesh` and B: `MaterialMeshBundle`
/// - `pbr_node_spawner` for A: `PbrMesh` and B: `PbrBundle` (specialized version of `material_mesh_node_spawner` with `Material` = `StandardMaterial`)
pub type BundleSpawner<A, B> =
    fn(assets: A, translation: Vec3, scale: Vec3, rotation: ModelRotation) -> B;

pub trait ComponentWrapper: Component + Clone + Sync + Send + 'static {}
impl<T: Component + Clone + Sync + Send + 'static> ComponentWrapper for T {}

#[derive(Clone, Component)]
pub struct NoComponents;

#[derive(Component)]
pub struct AssetSpawner<A: AssetHandles, B: Bundle, T: ComponentWrapper> {
    /// Link a `Model` via its [`ModelIndex`] to his spawnable assets (can be shared by multiple [`Generation`])
    pub assets: Arc<RulesModelsAssets<A, T>>,
    /// Size of a node in world units
    pub node_size: Vec3,
    /// Scale of the assets when spawned
    pub spawn_scale: Vec3,
    /// Called to spawn the appropriate [`Bundle`] for a node
    pub bundle_spawner: BundleSpawner<A, B>,
    /// Whether to offset the z coordinate of spawned nodes from the y coordinate (used for 2d ordering of sprites)
    pub z_offset_from_y: bool,
}

impl<A: AssetHandles, B: Bundle, T: ComponentWrapper> AssetSpawner<A, B, T> {
    /// Constructor for a `AssetSpawner`, `z_offset_from_y` defaults to `false`
    pub fn new(
        models_assets: RulesModelsAssets<A, T>,
        node_size: Vec3,
        spawn_scale: Vec3,
        bundle_spawner: BundleSpawner<A, B>,
    ) -> AssetSpawner<A, B, T> {
        Self {
            node_size,
            assets: Arc::new(models_assets),
            spawn_scale,
            bundle_spawner,
            z_offset_from_y: false,
        }
    }

    pub fn with_z_offset_from_y(mut self, z_offset_from_y: bool) -> Self {
        self.z_offset_from_y = z_offset_from_y;
        self
    }
}

/// Encapsulates a [`Generator`] and other information needed to correclty spawn assets
#[derive(Component)]
pub struct Generation<C: CoordinateSystem> {
    /// The generator that will produce the [`ModelInstance`]
    pub gen: Generator<C>,
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
    rotation: ModelRotation,
) -> SpriteBundle {
    SpriteBundle {
        texture,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_z(rotation.rad())),
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
    rotation: ModelRotation,
) -> SceneBundle {
    SceneBundle {
        scene,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rotation.rad())),
        ..default()
    }
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`Material`]
#[derive(Clone)]
pub struct MaterialMesh<M: Material> {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Material handle
    pub mat: Handle<M>,
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`StandardMaterial`]
///
/// Specialization of [`MaterialMesh`] with [`StandardMaterial`]
#[derive(Clone)]
pub struct PbrMesh {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Standard material handle
    pub mat: Handle<StandardMaterial>,
}

/// Utility [`BundleSpawner`]
///
/// Uses the y+ axis as the rotation axis
pub fn material_mesh_node_spawner<M: Material>(
    asset: MaterialMesh<M>,
    translation: Vec3,
    scale: Vec3,
    rotation: ModelRotation,
) -> MaterialMeshBundle<M> {
    MaterialMeshBundle {
        mesh: asset.mesh,
        material: asset.mat,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rotation.rad())),
        ..default()
    }
}

/// Utility [`BundleSpawner`], specialization of `material_mesh_node_spawner` with `Material` = `StandardMaterial`
///
/// Uses the y+ axis as the rotation axis
pub fn pbr_node_spawner(
    asset: PbrMesh,
    translation: Vec3,
    scale: Vec3,
    rotation: ModelRotation,
) -> PbrBundle {
    PbrBundle {
        mesh: asset.mesh,
        material: asset.mat,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rotation.rad())),
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
pub fn spawn_node<C: CoordinateSystem, A: AssetHandles, B: Bundle, T: ComponentWrapper>(
    commands: &mut Commands,
    gen_entity: Entity,
    generation: &Generation<C>,
    asset_spawner: &AssetSpawner<A, B, T>,
    instance: &ModelInstance,
    node_index: usize,
) {
    let node_assets_option = asset_spawner.assets.get(&instance.model_index);
    if node_assets_option.is_none() {
        return;
    }
    let node_assets = node_assets_option.unwrap();

    let pos = generation.gen.grid().get_position(node_index);
    for node_asset in node_assets {
        let offset = &node_asset.offset;
        // +0.5*scale to center the node because its center is at its origin
        let mut translation = Vec3::new(
            asset_spawner.node_size.x * (pos.x as f32 + offset.dx as f32 + 0.5),
            asset_spawner.node_size.y * (pos.y as f32 + offset.dy as f32 + 0.5),
            asset_spawner.node_size.z * (pos.z as f32 + offset.dz as f32 + 0.5),
        );

        if asset_spawner.z_offset_from_y {
            translation.z += asset_spawner.node_size.z
                * (1. - pos.y as f32 / generation.gen.grid().size_y() as f32);
        }

        let node_entity = commands
            .spawn((
                (asset_spawner.bundle_spawner)(
                    node_asset.handles.clone(),
                    translation,
                    asset_spawner.spawn_scale,
                    instance.rotation,
                ),
                SpawnedNode,
            ))
            .id();
        for component in node_asset.components.iter() {
            commands.entity(gen_entity).insert(component.clone());
        }
        commands.entity(gen_entity).add_child(node_entity);
    }
}
