use bevy::{
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query, Res, Resource},
    },
    hierarchy::BuildChildren,
    math::Vec3,
};
use ghx_proc_gen::{
    generator::model::ModelInstance,
    grid::{direction::CoordinateSystem, GridDefinition},
};

use self::assets::{AssetSpawner, AssetsBundleSpawner, ComponentSpawner};

/// Types to define and spawn assets
pub mod assets;

/// Debug plugin to run the generation & spawn assets automatically with different visualization options
pub mod debug_plugin;
/// Simple plugin to run the generation & spawn assets automatically
pub mod simple_plugin;

/// Adds default [`AssetsBundleSpawner`] implementations for common types.
///
/// **WARNING**: those default implementations each assume a specific `Rotation Axis` for the `Models` (Z+ for 2d, Y+ for 3d)
#[cfg(feature = "default-assets-bundle-spawners")]
pub mod default_bundles;

/// Marker for nodes spawned by a [`ghx_proc_gen::generator::Generator`]
#[derive(Component)]
pub struct SpawnedNode;

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
