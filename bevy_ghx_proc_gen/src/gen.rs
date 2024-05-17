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
use bevy_ghx_grid::ghx_grid::{coordinate_system::CoordinateSystem, grid::GridDefinition};
use ghx_proc_gen::{generator::model::ModelInstance, NodeIndex};

use self::assets::{AssetSpawner, AssetsBundleSpawner, ComponentSpawner};

/// Types to define and spawn assets
pub mod assets;

/// Debug plugin to run the generation & spawn assets automatically with different visualization options
#[cfg(feature = "debug-plugin")]
pub mod debug_plugin;
/// Simple plugin to run the generation & spawn assets automatically
#[cfg(feature = "simple-plugin")]
pub mod simple_plugin;

/// Adds default [`AssetsBundleSpawner`] implementations for common types.
///
/// **WARNING**: those default implementations each assume a specific `Rotation Axis` for the `Models` (Z+ for 2d, Y+ for 3d)
#[cfg(feature = "default-assets-bundle-spawners")]
pub mod default_bundles;

/// Used to mark a spawned gird node. Stores the [NodeIndex] of this node
#[derive(Component)]
pub struct GridNode(pub NodeIndex);

/// Flag for nodes spawned by a [`ghx_proc_gen::generator::Generator`]
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
        commands.entity(node).try_insert(B::default());
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
        commands.entity(node).try_insert(bundle_to_clone.clone());
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
    node_index: NodeIndex,
) {
    let node_assets = match asset_spawner.assets.get(&instance.model_index) {
        Some(node_assets) => node_assets,
        None => return,
    };

    let pos = grid.pos_from_index(node_index);
    for node_asset in node_assets {
        let offset = &node_asset.offset;
        let grid_offset = &node_asset.grid_offset;
        // + (0.5 * size) to center `translation` in the node
        let mut translation = Vec3::new(
            offset.x + asset_spawner.node_size.x * (pos.x as f32 + grid_offset.dx as f32 + 0.5),
            offset.y + asset_spawner.node_size.y * (pos.y as f32 + grid_offset.dy as f32 + 0.5),
            offset.z + asset_spawner.node_size.z * (pos.z as f32 + grid_offset.dz as f32 + 0.5),
        );

        if asset_spawner.z_offset_from_y {
            translation.z += asset_spawner.node_size.z * (1. - pos.y as f32 / grid.size_y() as f32);
        }

        let node_entity = commands.spawn((GridNode(node_index), SpawnedNode)).id();

        let node_entity_commands = &mut commands.entity(node_entity);
        node_asset.assets_bundle.insert_bundle(
            node_entity_commands,
            translation,
            asset_spawner.spawn_scale,
            instance.rotation,
        );
        for component in node_asset.components.iter() {
            component.insert(node_entity_commands);
        }
        commands.entity(gen_entity).add_child(node_entity);
    }
}
