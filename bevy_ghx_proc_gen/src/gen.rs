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
        system::{Commands, EntityCommands, Query, Res, Resource},
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
    generator::model::{ModelIndex, ModelInstance, ModelRotation},
    grid::{
        direction::{CoordinateSystem, GridDelta},
        GridDefinition,
    },
};

/// Debug plugin to run the generation & spawn assets automatically with different visualization options
pub mod debug_plugin;
/// Simple plugin to run the generation & spawn assets automatically
pub mod simple_plugin;

/// Marker for nodes spawned by a [`ghx_proc_gen::generator::Generator`]
#[derive(Component)]
pub struct SpawnedNode;

/// Defines a struct which can spawn an assets [`Bundle`] (for example, a [`SpriteBundle`], a [`PbrBundle`], a [`SceneBundle`], ...).
pub trait AssetsBundleSpawner: Sync + Send + 'static {
    /// From the `AssetsBundleSpawner` own data, a position, a scale and a rotation, inserts a [`Bundle`] into the spawned node `Entity`
    fn insert_bundle(
        &self,
        command: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    );
}

/// Represents spawnable asset(s) & component(s) for a model.
///
/// They will be spawned every time this model is generated. One `ModelAsset` will spawn exactly one [`Entity`].
#[derive(Clone)]
pub struct ModelAsset<A: AssetsBundleSpawner, T: ComponentSpawner = NoComponents> {
    /// Stores handle(s) to the asset(s) and spawns their bundle
    pub assets_bundle: A,
    /// Optionnal vector of [`ComponentSpawner`] that will be spawned for this model
    pub components: Vec<T>,
    /// Offset from the generated grid node. The asset will be instantiated at `generated_node_grid_pos + offset`
    pub offset: GridDelta,
}

/// Defines a map which links a `Model` via its [`ModelIndex`] to his spawnable [`ModelAsset`]
pub struct RulesModelsAssets<A: AssetsBundleSpawner, T: ComponentSpawner = NoComponents> {
    /// Only contains a ModelIndex if there are some assets for it. One model may have multiple [`ModelAsset`].
    map: HashMap<ModelIndex, Vec<ModelAsset<A, T>>>,
}
impl<A: AssetsBundleSpawner, T: ComponentSpawner> Deref for RulesModelsAssets<A, T> {
    type Target = HashMap<ModelIndex, Vec<ModelAsset<A, T>>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl<A: AssetsBundleSpawner, T: ComponentSpawner> DerefMut for RulesModelsAssets<A, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<A: AssetsBundleSpawner, T: ComponentSpawner> RulesModelsAssets<A, T> {
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
            assets_bundle: asset,
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

/// Trait used to represent a generic [`Component`]/[`Bundle`] container.
///
/// Can be used to store custom components in [`ModelAsset`].
pub trait ComponentSpawner: Sync + Send + 'static {
    /// Insert [`Component`] and/or [`Bundle`] into an [`Entity`]
    fn insert(&self, commands: &mut EntityCommands);
}

/// Default implementation of [`ComponentSpawner`] which does nothing.
///
/// `Insert` will not even be called if your [`ModelAsset`] don't have components.
#[derive(Clone)]
pub struct NoComponents;
impl ComponentSpawner for NoComponents {
    fn insert(&self, _commands: &mut EntityCommands) {}
}

/// Stores information needed to spawn assets from a [`ghx_proc_gen::generator::Generator`]
#[derive(Component)]
pub struct AssetSpawner<A: AssetsBundleSpawner, T: ComponentSpawner = NoComponents> {
    /// Link a `Model` via its [`ModelIndex`] to his spawnable assets (can be shared by multiple [`AssetSpawner`])
    pub assets: Arc<RulesModelsAssets<A, T>>,
    /// Size of a node in world units
    pub node_size: Vec3,
    /// Scale of the assets when spawned
    pub spawn_scale: Vec3,
    /// Whether to offset the z coordinate of spawned nodes from the y coordinate (used for 2d ordering of sprites)
    pub z_offset_from_y: bool,
}

impl<A: AssetsBundleSpawner, T: ComponentSpawner> AssetSpawner<A, T> {
    /// Constructor for a `AssetSpawner`, `z_offset_from_y` defaults to `false`
    pub fn new(
        models_assets: RulesModelsAssets<A, T>,
        node_size: Vec3,
        spawn_scale: Vec3,
    ) -> AssetSpawner<A, T> {
        Self {
            node_size,
            assets: Arc::new(models_assets),
            spawn_scale,
            z_offset_from_y: false,
        }
    }

    /// Sets the `z_offset_from_y` value
    pub fn with_z_offset_from_y(mut self, z_offset_from_y: bool) -> Self {
        self.z_offset_from_y = z_offset_from_y;
        self
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

impl AssetsBundleSpawner for Handle<Image> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(SpriteBundle {
            texture: self.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_z(rotation.rad())),
            ..default()
        });
    }
}

impl AssetsBundleSpawner for Handle<Scene> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(SceneBundle {
            scene: self.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            ..default()
        });
    }
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`Material`]
#[derive(Clone)]
pub struct MaterialMesh<M: Material> {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Material handle
    pub material: Handle<M>,
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`StandardMaterial`]
///
/// Specialization of [`MaterialMesh`] with [`StandardMaterial`]
#[derive(Clone)]
pub struct PbrMesh {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Standard material handle
    pub material: Handle<StandardMaterial>,
}

impl<M: Material> AssetsBundleSpawner for MaterialMesh<M> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(MaterialMeshBundle {
            mesh: self.mesh.clone(),
            material: self.material.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            ..default()
        });
    }
}

impl AssetsBundleSpawner for PbrMesh {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(PbrBundle {
            mesh: self.mesh.clone(),
            material: self.material.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            ..default()
        });
    }
}

/// Utility system to spawn grid nodes. Can work for multiple asset types.
///
/// Used by [`simple_plugin::ProcGenSimplePlugin`] and [`debug_plugin::ProcGenDebugPlugin`] to spawn assets automatically.
///
/// ### Examples
///
/// Spawn 3d models (gltf) assets with a `Cartesian3D` grid
/// ```ignore
/// spawn_node::<Cartesian3D, Handle<Scene>>(...);
/// ```
/// Spawn 2d sprites (png, ...) assets with a `Cartesian3D` grid
/// ```ignore
/// spawn_node::<Cartesian3D, Handle<Image>>(...);
/// ```
pub fn spawn_node<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner>(
    commands: &mut Commands,
    gen_entity: Entity,
    grid: &GridDefinition<C>,
    asset_spawner: &AssetSpawner<A, T>,
    instance: &ModelInstance,
    node_index: usize,
) {
    let node_assets_option = asset_spawner.assets.get(&instance.model_index);
    if node_assets_option.is_none() {
        return;
    }
    let node_assets = node_assets_option.unwrap();

    let pos = grid.get_position(node_index);
    for node_asset in node_assets {
        let offset = &node_asset.offset;
        // +0.5*scale to center the node because its center is at its origin
        let mut translation = Vec3::new(
            asset_spawner.node_size.x * (pos.x as f32 + offset.dx as f32 + 0.5),
            asset_spawner.node_size.y * (pos.y as f32 + offset.dy as f32 + 0.5),
            asset_spawner.node_size.z * (pos.z as f32 + offset.dz as f32 + 0.5),
        );

        if asset_spawner.z_offset_from_y {
            translation.z += asset_spawner.node_size.z * (1. - pos.y as f32 / grid.size_y() as f32);
        }

        let node_entity = commands.spawn(SpawnedNode).id();

        let node_entity_commands = &mut commands.entity(node_entity);
        node_asset.assets_bundle.insert_bundle(
            node_entity_commands,
            translation,
            asset_spawner.spawn_scale,
            instance.rotation,
        );
        for component in node_asset.components.iter() {
            component.insert(&mut commands.entity(node_entity));
        }
        commands.entity(gen_entity).add_child(node_entity);
    }
}
