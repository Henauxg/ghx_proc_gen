use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PluginGroup, PluginGroupBuilder, Update},
    ecs::{
        entity::Entity,
        query::Added,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, ResMut},
    },
    platform::collections::HashSet,
    prelude::Resource,
};

#[cfg(feature = "log")]
use bevy::log::{info, warn};

use ghx_proc_gen::{
    generator::Generator,
    ghx_grid::cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid},
    GeneratorError,
};

use crate::{spawner_plugin::ProcGenSpawnerPlugin, BundleInserter, GridGeneratedEvent};

/// A simple [`Plugin`] that automatically detects any [`Entity`] with a [`Generator`] `Component` and tries to run the contained generator once per frame until it succeeds.
///
/// Once the generation is successful, the plugin will spawn the generated nodes assets.
pub struct ProcGenSimpleRunnerPlugin<C: CartesianCoordinates> {
    typestate: PhantomData<C>,
}
impl<C: CartesianCoordinates> Plugin for ProcGenSimpleRunnerPlugin<C> {
    fn build(&self, app: &mut App) {
        app.insert_resource(PendingGenerations::default());

        app.add_systems(
            Update,
            (register_new_generations::<C>, generate_and_spawn::<C>).chain(),
        );
    }
}
impl<C: CartesianCoordinates> ProcGenSimpleRunnerPlugin<C> {
    /// Constructor
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

/// A group of plugins that combines simple generation and nodes spawning
#[derive(Default)]
pub struct ProcGenSimplePlugins<C: CartesianCoordinates, A: BundleInserter> {
    typestate: PhantomData<(C, A)>,
}
impl<C: CartesianCoordinates, A: BundleInserter> PluginGroup for ProcGenSimplePlugins<C, A> {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ProcGenSimpleRunnerPlugin::<C>::new())
            .add(ProcGenSpawnerPlugin::<C, A>::new())
    }
}

/// Resource used by [`ProcGenSimpleRunnerPlugin`] to track generations that are yet to generate a result
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

/// System used by [`ProcGenSimpleRunnerPlugin`] to track entities with newly added [`Generator`] components
pub fn register_new_generations<C: CartesianCoordinates>(
    mut pending_generations: ResMut<PendingGenerations>,
    mut new_generations: Query<Entity, Added<Generator<C, CartesianGrid<C>>>>,
) {
    for gen_entity in new_generations.iter_mut() {
        pending_generations.pendings.insert(gen_entity);
    }
}

/// System used by [`ProcGenSimpleRunnerPlugin`] to run generators
pub fn generate_and_spawn<C: CartesianCoordinates>(
    mut commands: Commands,
    mut pending_generations: ResMut<PendingGenerations>,
    mut generations: Query<&mut Generator<C, CartesianGrid<C>>>,
) {
    let mut generations_done = vec![];
    for &gen_entity in pending_generations.pendings.iter() {
        if let Ok(mut generation) = generations.get_mut(gen_entity) {
            match generation.generate_grid() {
                Ok((gen_info, grid_data)) => {
                    #[cfg(feature = "log")]
                    info!(
                        "Generation {:?} done, try_count: {}, seed: {}; grid: {}",
                        gen_entity,
                        gen_info.try_count,
                        generation.seed(),
                        generation.grid()
                    );
                    commands.trigger_targets(GridGeneratedEvent(grid_data), gen_entity);
                    generations_done.push(gen_entity);
                }
                Err(GeneratorError { node_index }) => {
                    #[cfg(feature = "log")]
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
