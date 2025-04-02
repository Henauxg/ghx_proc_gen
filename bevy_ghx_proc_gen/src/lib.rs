#![warn(missing_docs)]

//! This library encapsulates (and re-exports) the "ghx_proc_gen" library for 2D & 3D procedural generation with Model synthesis/Wave function Collapse, for a Bevy usage.
//! Also provide grid utilities to manipulate & debug 2d & 3d grid data with Bevy.

use bevy::{
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        event::Event,
        query::Added,
        system::{Commands, Query, Res, Resource},
    },
    hierarchy::BuildChildren,
    math::Vec3,
    prelude::{Deref, DerefMut, Without},
    utils::HashSet,
};
use ghx_proc_gen::{
    generator::{
        model::{ModelIndex, ModelInstance},
        GeneratedNode,
    },
    ghx_grid::{
        cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid},
        grid::GridData,
    },
    NodeIndex,
};

use assets::BundleInserter;
use spawner_plugin::NodesSpawner;

pub use bevy_ghx_grid;
pub use ghx_proc_gen as proc_gen;

/// Types to define and spawn assets
pub mod assets;

/// Debug plugin to run the generation with different visualization options
#[cfg(feature = "debug-plugin")]
pub mod debug_plugin;
/// Simple plugin to run the generation
#[cfg(feature = "simple-plugin")]
pub mod simple_plugin;
/// Plugin to automatically spawn generated nodes
pub mod spawner_plugin;

/// Adds default [`BundleInserter`] implementations for some common types.
///
/// **WARNING**: those default implementations each assume a specific `Rotation Axis` for the `Models` (Z+ for 2d, Y+ for 3d)
#[cfg(feature = "default-assets-bundle-spawners")]
pub mod default_bundles;

#[cfg(feature = "egui-edit")]
pub use bevy_egui;

/// The generation with the specified entity was fully generated
#[derive(Event, Clone)]
pub struct GridGeneratedEvent<C: CartesianCoordinates>(
    pub GridData<C, ModelInstance, CartesianGrid<C>>,
);

/// The generation with the specified entity was reinitialized
#[derive(Event, Clone, Debug)]
pub struct GenerationResetEvent;

/// The generation with the specified entity was updated on the specified node
#[derive(Event, Clone, Debug)]
pub struct NodesGeneratedEvent(pub Vec<GeneratedNode>);

/// Used to mark a node spawned by a [`ghx_proc_gen::generator::Generator`]. Stores the [NodeIndex] of this node
#[derive(Component)]
pub struct GridNode(pub NodeIndex);

/// Main component marker for a cursor target
#[derive(Component)]
pub struct CursorTarget;

/// Component used to store model indexes of models with no assets.
///
/// Can be used for special handling of those models (skip their generation when stepping, ...).
#[derive(Component, Default, Deref, DerefMut)]
pub struct VoidNodes(pub HashSet<ModelIndex>);

/// Utility system. Adds a [`Bundle`] (or a [`Component`]) to every [`Entity`] that has [`GridNode`] Component (this is the case of nodes spawned by the `spawn_node` system). The `Bundle` will have its default value.
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
    spawned_nodes: Query<Entity, (Added<GridNode>, Without<CursorTarget>)>,
) {
    for node in spawned_nodes.iter() {
        commands.entity(node).try_insert(B::default());
    }
}

/// Utility system. Adds a [`Bundle`] (or a [`Component`]) to every [`Entity`] that has [`GridNode`] Component (this is the case of nodes spawned by the `spawn_node` system). The `Bundle` will be cloned from a `Resource`
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
    spawned_nodes: Query<Entity, (Added<GridNode>, Without<CursorTarget>)>,
) {
    for node in spawned_nodes.iter() {
        commands.entity(node).try_insert(bundle_to_clone.clone());
    }
}

/// Utility function to spawn grid nodes. Can work for multiple asset types.
///
/// Used by [`simple_plugin::ProcGenSimpleRunnerPlugin`] and [`debug_plugin::ProcGenDebugRunnerPlugin`] to spawn assets automatically.
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
pub fn spawn_node<C: CartesianCoordinates, A: BundleInserter>(
    commands: &mut Commands,
    gen_entity: Entity,
    grid: &CartesianGrid<C>,
    node_spawner: &NodesSpawner<A>,
    instance: &ModelInstance,
    node_index: NodeIndex,
) {
    let node_assets = match node_spawner.assets.get(&instance.model_index) {
        Some(node_assets) => node_assets,
        None => return,
    };

    let pos = grid.pos_from_index(node_index);
    for node_asset in node_assets {
        let offset = &node_asset.world_offset;
        let grid_offset = &node_asset.grid_offset;
        // + (0.5 * size) to center `translation` in the node
        let mut translation = Vec3::new(
            offset.x + node_spawner.node_size.x * (pos.x as f32 + grid_offset.dx as f32 + 0.5),
            offset.y + node_spawner.node_size.y * (pos.y as f32 + grid_offset.dy as f32 + 0.5),
            offset.z + node_spawner.node_size.z * (pos.z as f32 + grid_offset.dz as f32 + 0.5),
        );

        if node_spawner.z_offset_from_y {
            translation.z += node_spawner.node_size.z * (1. - pos.y as f32 / grid.size_y() as f32);
        }

        let node_entity = commands.spawn(GridNode(node_index)).id();
        let node_entity_commands = &mut commands.entity(node_entity);

        node_asset.assets_bundle.insert_bundle(
            node_entity_commands,
            translation,
            node_spawner.spawn_scale,
            instance.rotation,
        );
        (node_asset.spawn_commands)(node_entity_commands);

        commands.entity(gen_entity).add_child(node_entity);
    }
}

macro_rules! add_named_observer {
    ($system: expr, $app: expr) => {
        $app.world_mut().spawn((
            bevy::core::Name::new(stringify!($system)),
            bevy::ecs::observer::Observer::new($system),
        ))
    };
}
pub(crate) use add_named_observer;
