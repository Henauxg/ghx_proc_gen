use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Update},
    asset::Asset,
    ecs::{
        bundle::Bundle,
        entity::Entity,
        query::Added,
        system::{Commands, Query},
    },
    log::{info, warn},
};
use ghx_proc_gen::GenerationError;

use crate::{gen::spawn_node, grid::SharableCoordSystem};

use super::Generation;

pub struct ProcGenSimplePlugin<C: SharableCoordSystem, A: Asset, B: Bundle> {
    typestate: PhantomData<(C, A, B)>,
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> Plugin for ProcGenSimplePlugin<C, A, B> {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, generate_and_spawn::<C, A, B>);
    }
}

impl<C: SharableCoordSystem, A: Asset, B: Bundle> ProcGenSimplePlugin<C, A, B> {
    pub fn new() -> Self {
        Self {
            typestate: PhantomData,
        }
    }
}

pub fn generate_and_spawn<C: SharableCoordSystem, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut new_generations: Query<(Entity, &mut Generation<C, A, B>), Added<Generation<C, A, B>>>,
) {
    for (gen_entity, mut generation) in new_generations.iter_mut() {
        match generation.gen.generate_collected() {
            Ok(grid_data) => {
                info!(
                    "Generation done, seed: {}; grid: {}",
                    generation.gen.get_seed(),
                    generation.gen.grid()
                );
                for (node_index, node) in grid_data.nodes().iter().enumerate() {
                    spawn_node(&mut commands, gen_entity, &generation, node, node_index);
                }
            }
            Err(GenerationError { node_index }) => {
                warn!(
                    "Generation Failed at node {}, seed: {}; grid: {}",
                    node_index,
                    generation.gen.get_seed(),
                    generation.gen.grid()
                );
            }
        }
    }
}
