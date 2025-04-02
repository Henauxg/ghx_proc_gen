use std::{marker::PhantomData, sync::Arc};

use bevy::{
    app::{App, Plugin},
    ecs::{
        component::Component,
        query::Without,
        system::{Commands, Query},
        world::OnAdd,
    },
    math::Vec3,
    prelude::{Children, DespawnRecursiveExt, Entity, Trigger, With},
    render::view::Visibility,
    utils::HashSet,
};
use ghx_proc_gen::{
    generator::Generator,
    ghx_grid::cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid},
};

use crate::{
    add_named_observer, spawn_node, GenerationResetEvent, GridGeneratedEvent, NodesGeneratedEvent,
    VoidNodes,
};

use super::{assets::ModelsAssets, BundleInserter, GridNode};

/// Plugins that automatically spawn entites & assets for generated nodes
#[derive(Default)]
pub struct ProcGenSpawnerPlugin<C: CartesianCoordinates, A: BundleInserter> {
    typestate: PhantomData<(C, A)>,
}

impl<C: CartesianCoordinates, A: BundleInserter> Plugin for ProcGenSpawnerPlugin<C, A> {
    fn build(&self, app: &mut App) {
        add_named_observer!(default_grid_spawner::<C, A>, app);
        add_named_observer!(default_node_despawner::<C>, app);
        add_named_observer!(default_node_spawner::<C, A>, app);
        add_named_observer!(insert_void_nodes_to_new_generations::<C, A>, app);
    }
}

impl<C: CartesianCoordinates, A: BundleInserter> ProcGenSpawnerPlugin<C, A> {
    /// Simple constructor
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

/// Stores information needed to spawn node assets from a [`ghx_proc_gen::generator::Generator`]
#[derive(Component, Clone, Debug)]
#[require(Visibility)]
pub struct NodesSpawner<A: BundleInserter> {
    /// Link a `Model` to his spawnable assets via its [`ghx_proc_gen::generator::model::ModelIndex`] (can be shared by multiple [`NodesSpawner`])
    pub assets: Arc<ModelsAssets<A>>,
    /// Size of a node in world units
    pub node_size: Vec3,
    /// Scale of the assets when spawned
    pub spawn_scale: Vec3,
    /// Whether to offset the z coordinate of spawned nodes from the y coordinate (used for 2d ordering of sprites)
    pub z_offset_from_y: bool,
}

impl<A: BundleInserter> NodesSpawner<A> {
    /// Constructor, `z_offset_from_y` defaults to `false`
    pub fn new(
        models_assets: ModelsAssets<A>,
        node_size: Vec3,
        spawn_scale: Vec3,
    ) -> NodesSpawner<A> {
        Self {
            assets: Arc::new(models_assets),
            node_size,
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

/// Simple observer system that calculates and add a [`VoidNodes`] component for generator entites which don't have one yet.
pub fn insert_void_nodes_to_new_generations<C: CartesianCoordinates, A: BundleInserter>(
    trigger: Trigger<OnAdd, Generator<C, CartesianGrid<C>>>,
    mut commands: Commands,
    mut new_generations: Query<
        (
            Entity,
            &mut Generator<C, CartesianGrid<C>>,
            &NodesSpawner<A>,
        ),
        Without<VoidNodes>,
    >,
) {
    let Ok((gen_entity, generation, nodes_spawner)) = new_generations.get_mut(trigger.entity())
    else {
        return;
    };

    let mut void_nodes = HashSet::new();
    for model_index in 0..generation.rules().original_models_count() {
        if !nodes_spawner.assets.contains_key(&model_index) {
            void_nodes.insert(model_index);
        }
    }
    commands.entity(gen_entity).insert(VoidNodes(void_nodes));
}

/// Spawns every nodes of a fully generated generator entity as children
pub fn default_grid_spawner<C: CartesianCoordinates, A: BundleInserter>(
    trigger: Trigger<GridGeneratedEvent<C>>,
    mut commands: Commands,
    generators: Query<(&NodesSpawner<A>, &Generator<C, CartesianGrid<C>>)>,
) {
    let gen_entity = trigger.entity();
    if let Ok((asset_spawner, generator)) = generators.get(gen_entity) {
        for (node_index, model_instance) in trigger.event().0.iter().enumerate() {
            spawn_node(
                &mut commands,
                gen_entity,
                &generator.grid(),
                asset_spawner,
                model_instance,
                node_index,
            );
        }
    }
}

/// Despawns every children nodes of a generator entity
pub fn default_node_despawner<C: CartesianCoordinates>(
    trigger: Trigger<GenerationResetEvent>,
    mut commands: Commands,
    generators: Query<&Children>,
    existing_nodes: Query<Entity, With<GridNode>>,
) {
    if let Ok(children) = generators.get(trigger.entity()) {
        for &child in children.iter() {
            if let Ok(node) = existing_nodes.get(child) {
                commands.entity(node).despawn_recursive();
            }
        }
    }
}

/// Spawns a collection of nodes of a generator entity as children
pub fn default_node_spawner<C: CartesianCoordinates, A: BundleInserter>(
    trigger: Trigger<NodesGeneratedEvent>,
    mut commands: Commands,
    generators: Query<(&NodesSpawner<A>, &Generator<C, CartesianGrid<C>>)>,
) {
    let gen_entity = trigger.entity();
    if let Ok((node_spawner, generator)) = generators.get(gen_entity) {
        for node in trigger.event().0.iter() {
            spawn_node(
                &mut commands,
                gen_entity,
                &generator.grid(),
                node_spawner,
                &node.model_instance,
                node.node_index,
            );
        }
    }
}
