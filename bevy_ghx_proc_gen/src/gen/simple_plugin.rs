use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Update},
    asset::Asset,
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
use ghx_proc_gen::GenerationError;

use crate::{gen::spawn_node, grid::SharableCoordSystem};

use super::Generation;

pub struct ProcGenSimplePlugin<C: SharableCoordSystem, A: Asset, B: Bundle> {
    typestate: PhantomData<(C, A, B)>,
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> Plugin for ProcGenSimplePlugin<C, A, B> {
    fn build(&self, app: &mut App) {
        app.insert_resource(PendingGenerations::default());
        app.add_systems(
            Update,
            (
                register_new_generation::<C, A, B>,
                generate_and_spawn::<C, A, B>,
            )
                .chain(),
        );
    }
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> ProcGenSimplePlugin<C, A, B> {
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

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

pub fn register_new_generation<C: SharableCoordSystem, A: Asset, B: Bundle>(
    mut pending_generations: ResMut<PendingGenerations>,
    mut new_generations: Query<Entity, Added<Generation<C, A, B>>>,
) {
    for gen_entity in new_generations.iter_mut() {
        pending_generations.pendings.insert(gen_entity);
    }
}

pub fn generate_and_spawn<C: SharableCoordSystem, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut pending_generations: ResMut<PendingGenerations>,
    mut generations: Query<&mut Generation<C, A, B>>,
) {
    let mut generations_done = vec![];
    for &gen_entity in pending_generations.pendings.iter() {
        if let Ok(mut generation) = generations.get_mut(gen_entity) {
            match generation.gen.generate_collected() {
                Ok(grid_data) => {
                    info!(
                        "Generation {:?} done, seed: {}; grid: {}",
                        gen_entity,
                        generation.gen.get_seed(),
                        generation.gen.grid()
                    );
                    for (node_index, node) in grid_data.nodes().iter().enumerate() {
                        spawn_node(&mut commands, gen_entity, &generation, node, node_index);
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
