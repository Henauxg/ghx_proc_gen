use std::marker::PhantomData;
use crate::gen::CartesianCoordinates;

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        entity::Entity,
        query::Added,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, ResMut, Resource},
    },
    log::{info, warn},
    utils::HashSet,
};
use bevy_ghx_grid::ghx_grid::coordinate_system::CoordinateSystem;
use ghx_proc_gen::{generator::Generator, GeneratorError};

use crate::gen::spawn_node;

use super::{assets::NoComponents, AssetSpawner, AssetsBundleSpawner, ComponentSpawner};

/// A simple [`Plugin`] that automatically detects any [`Entity`] with a [`Generator`] `Component` and tries to run the contained generator once per frame until it succeeds.
///
/// Once the generation is successful, the plugin will spawn the generated nodes assets.
pub struct ProcGenSimplePlugin<
    C: CoordinateSystem + CartesianCoordinates,
    A: AssetsBundleSpawner,
    T: ComponentSpawner = NoComponents,
> {
    typestate: PhantomData<(C, A, T)>,
}

impl<C: CoordinateSystem + CartesianCoordinates, A: AssetsBundleSpawner, T: ComponentSpawner> Plugin
    for ProcGenSimplePlugin<C, A, T>
{
    fn build(&self, app: &mut App) {
        app.insert_resource(PendingGenerations::default());
        app.add_systems(
            Update,
            (register_new_generations::<C>, generate_and_spawn::<C, A, T>).chain(),
        );
    }
}

impl<C: CoordinateSystem + CartesianCoordinates, A: AssetsBundleSpawner, T: ComponentSpawner>
    ProcGenSimplePlugin<C, A, T>
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

/// System used by [`ProcGenSimplePlugin`] to track entities with newly added [`Generator`] components
pub fn register_new_generations<C: CoordinateSystem + CartesianCoordinates>(
    mut pending_generations: ResMut<PendingGenerations>,
    mut new_generations: Query<Entity, Added<Generator<C>>>,
) {
    for gen_entity in new_generations.iter_mut() {
        pending_generations.pendings.insert(gen_entity);
    }
}

/// System used by [`ProcGenSimplePlugin`] to run generators and spawn their node's assets
pub fn generate_and_spawn<C: CoordinateSystem + CartesianCoordinates, A: AssetsBundleSpawner, T: ComponentSpawner>(
    mut commands: Commands,
    mut pending_generations: ResMut<PendingGenerations>,
    mut generations: Query<(&mut Generator<C>, &AssetSpawner<A, T>)>,
) {
    let mut generations_done = vec![];
    for &gen_entity in pending_generations.pendings.iter() {
        if let Ok((mut generation, asset_spawner)) = generations.get_mut(gen_entity) {
            match generation.generate_grid() {
                Ok((gen_info, grid_data)) => {
                    info!(
                        "Generation {:?} done, try_count: {}, seed: {}; grid: {}",
                        gen_entity,
                        gen_info.try_count,
                        generation.seed(),
                        generation.grid()
                    );
                    for (node_index, node) in grid_data.iter().enumerate() {
                        spawn_node(
                            &mut commands,
                            gen_entity,
                            &generation.grid(),
                            asset_spawner,
                            node,
                            node_index,
                        );
                    }
                    generations_done.push(gen_entity);
                }
                Err(GeneratorError { node_index }) => {
                    warn!(
                        "Generation {:?} failed at node {}, seed: {}; grid: {}",
                        gen_entity,
                        node_index,
                        generation.seed(),
                        generation.grid()
                    );
                }
            }
        }
    }
    for gen_entity in generations_done {
        pending_generations.pendings.remove(&gen_entity);
    }
}
