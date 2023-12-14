use std::collections::HashMap;

use bevy::{
    asset::Handle,
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::DespawnRecursiveExt,
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    log::info,
    math::Quat,
    render::{texture::Image, view::Visibility},
    sprite::SpriteBundle,
    time::{Time, Timer},
    transform::components::Transform,
    utils::default,
};
use bevy_ghx_proc_gen::{
    grid::{DebugGridView, SharableDirectionSet},
    proc_gen::{
        generator::{
            node::GeneratedNode,
            observer::{GenerationUpdate, QueuedObserver},
            GenerationStatus, Generator,
        },
        grid::GridDefinition,
    },
};

#[derive(PartialEq, Eq)]
pub enum GenerationViewMode {
    StepByStep(u64),
    StepByStepPaused,
    Final,
}

#[derive(Resource)]
pub struct Generation<T: SharableDirectionSet> {
    pub models_assets: HashMap<usize, Handle<Image>>,
    pub gen: Generator<T>,
    pub observer: QueuedObserver,
    /// Size of a node in world units
    pub node_size: f32,
}

#[derive(Resource)]
pub struct GenerationTimer(pub Timer);

#[derive(Component)]
pub struct SpawnedNode;

pub fn spawn_node<T: SharableDirectionSet>(
    commands: &mut Commands,
    models_assets: &HashMap<usize, Handle<Image>>,
    grid: &GridDefinition<T>,
    node: &GeneratedNode,
    node_index: usize,
    tile_size: f32,
) {
    info!("Spawning {:?} at node index {}", node, node_index);
    if let Some(asset) = models_assets.get(&node.model_index) {
        let x_offset = tile_size * grid.size_x() as f32 / 2.;
        let y_offset = tile_size * grid.size_y() as f32 / 2.;
        let pos = grid.get_position(node_index);
        commands.spawn((
            SpriteBundle {
                texture: asset.clone(),
                transform: Transform::from_xyz(
                    tile_size * pos.x as f32 - x_offset,
                    tile_size * pos.y as f32 - y_offset,
                    pos.z as f32,
                )
                .with_rotation(Quat::from_rotation_z(f32::to_radians(
                    node.rotation.value() as f32,
                ))),
                ..default()
            },
            SpawnedNode,
        ));
    }
}

fn select_and_propagate<T: SharableDirectionSet>(
    commands: &mut Commands,
    generation: &mut ResMut<Generation<T>>,
    nodes: Query<Entity, With<SpawnedNode>>,
) {
    match generation.gen.select_and_propagate() {
        Ok(status) => match status {
            GenerationStatus::Ongoing => (),
            GenerationStatus::Done => info!("Generation done"),
        },
        Err(_) => {
            info!("Generation Failed")
        }
    }
    // Process the observer queue even if generation failed
    let updates = generation.observer.dequeue_all();
    // Buffer the nodes to spawn in case a generation failure invalidate some.
    let mut nodes_to_spawn = vec![];
    let mut despawn_nodes = false;
    for update in updates {
        match update {
            GenerationUpdate::Generated {
                node_index,
                generated_node,
            } => {
                nodes_to_spawn.push((node_index, generated_node));
            }
            GenerationUpdate::Reinitialized | GenerationUpdate::Failed => {
                nodes_to_spawn.clear();
                despawn_nodes = true;
            }
        }
    }
    if despawn_nodes {
        for entity in nodes.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }

    for (node_index, generated_node) in nodes_to_spawn {
        spawn_node(
            commands,
            &generation.models_assets,
            generation.gen.grid(),
            &generated_node,
            node_index,
            generation.node_size,
        );
    }
}

pub fn step_by_step_input_update<T: SharableDirectionSet>(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    mut generation: ResMut<Generation<T>>,
    nodes: Query<Entity, With<SpawnedNode>>,
) {
    if keys.pressed(KeyCode::Space) || buttons.just_pressed(MouseButton::Left) {
        select_and_propagate(&mut commands, &mut generation, nodes);
    }
}

pub fn step_by_step_timed_update<T: SharableDirectionSet>(
    mut commands: Commands,
    mut generation: ResMut<Generation<T>>,
    mut timer: ResMut<GenerationTimer>,
    time: Res<Time>,
    nodes: Query<Entity, With<SpawnedNode>>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        select_and_propagate(&mut commands, &mut generation, nodes);
    }
}

pub fn toggle_debug_grid_visibility(
    keys: Res<Input<KeyCode>>,
    mut debug_grids: Query<&mut Visibility, With<DebugGridView>>,
) {
    if keys.just_pressed(KeyCode::F1) {
        for mut view_visibility in debug_grids.iter_mut() {
            *view_visibility = match *view_visibility {
                Visibility::Inherited => Visibility::Hidden,
                Visibility::Hidden => Visibility::Visible,
                Visibility::Visible => Visibility::Hidden,
            }
        }
    }
}
