use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        bundle::Bundle,
        entity::Entity,
        query::Added,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, ResMut, Resource},
    },
    log::{info, warn},
    utils::HashSet,
};
use ghx_proc_gen::{grid::direction::CoordinateSystem, GenerationError};

use crate::{gen::spawn_node, ComponentWrapper};

use super::{AssetHandles, AssetSpawner, Generation, NoComponents};

/// A simple [`Plugin`] that automatically detects any [`Entity`] with a [`Generation`] `Component` and tries to run the contained generator once per frame until it succeeds.
///
/// Once the generation is successful, the plugin will spawn the generated nodes assets.
pub struct ProcGenSimplePlugin<
    C: CoordinateSystem,
    A: AssetHandles,
    B: Bundle,
    T: ComponentWrapper = NoComponents,
> {
    typestate: PhantomData<(C, A, B, T)>,
}

impl<C: CoordinateSystem, A: AssetHandles, B: Bundle, T: ComponentWrapper> Plugin
    for ProcGenSimplePlugin<C, A, B, T>
{
    fn build(&self, app: &mut App) {
        app.insert_resource(PendingGenerations::default());
        app.add_systems(
            Update,
            (
                register_new_generations::<C>,
                generate_and_spawn::<C, A, B, T>,
            )
                .chain(),
        );
    }
}

impl<C: CoordinateSystem, A: AssetHandles, B: Bundle, T: ComponentWrapper>
    ProcGenSimplePlugin<C, A, B, T>
{
    /// Constructor
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

/// Resource used by [`ProcGenSimplePlugin`] to track generations that are yet to generate a result
#[derive(Resource)]
pub struct PendingGenerations {
    pendings: HashSet<Entity>,
}

impl Default for PendingGenerations {
    fn default() -> Self {
        Self {
            pendings: Default::default(),
        }
    }
}

/// System used by [`ProcGenSimplePlugin`] to track entities with newly added [`Generation`] components
pub fn register_new_generations<C: CoordinateSystem>(
    mut pending_generations: ResMut<PendingGenerations>,
    mut new_generations: Query<Entity, Added<Generation<C>>>,
) {
    for gen_entity in new_generations.iter_mut() {
        pending_generations.pendings.insert(gen_entity);
    }
}

/// System used by [`ProcGenSimplePlugin`] to run generators and spawn their node's assets
pub fn generate_and_spawn<C: CoordinateSystem, A: AssetHandles, B: Bundle, T: ComponentWrapper>(
    mut commands: Commands,
    mut pending_generations: ResMut<PendingGenerations>,
    mut generations: Query<(&mut Generation<C>, &AssetSpawner<A, B, T>)>,
) {
    let mut generations_done = vec![];
    for &gen_entity in pending_generations.pendings.iter() {
        if let Ok((mut generation, asset_spawner)) = generations.get_mut(gen_entity) {
            match generation.gen.generate_collected() {
                Ok(grid_data) => {
                    info!(
                        "Generation {:?} done, seed: {}; grid: {}",
                        gen_entity,
                        generation.gen.get_seed(),
                        generation.gen.grid()
                    );
                    for (node_index, node) in grid_data.nodes().iter().enumerate() {
                        spawn_node(
                            &mut commands,
                            gen_entity,
                            &generation.gen.grid(),
                            asset_spawner,
                            node,
                            node_index,
                        );
                    }
                    generations_done.push(gen_entity);
                }
                Err(GenerationError { node_index }) => {
                    warn!(
                        "Generation {:?} failed at node {}, seed: {}; grid: {}",
                        gen_entity,
                        node_index,
                        generation.gen.get_seed(),
                        generation.gen.grid()
                    );
                }
            }
        }
    }
    for gen_entity in generations_done {
        pending_generations.pendings.remove(&gen_entity);
    }
}
