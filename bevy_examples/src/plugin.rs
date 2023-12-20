use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, PostStartup, Update},
    asset::{Asset, Handle},
    ecs::{
        bundle::Bundle,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    log::info,
    math::{Quat, Vec3},
    render::texture::Image,
    scene::{Scene, SceneBundle},
    sprite::SpriteBundle,
    time::{Time, Timer, TimerMode},
    transform::components::Transform,
    utils::default,
};
use bevy_ghx_proc_gen::{
    grid::SharableDirectionSet,
    proc_gen::generator::{node::GeneratedNode, observer::GenerationUpdate, GenerationStatus},
};

use crate::{
    anim::animate_spawning_nodes_scale, Generation, GenerationTimer, GenerationViewMode,
    SpawnedNode,
};

pub struct ProcGenExamplesPlugin<T: SharableDirectionSet, A: Asset, B: Bundle> {
    generation_view_mode: GenerationViewMode,
    typestate: PhantomData<(T, A, B)>,
}

impl<T: SharableDirectionSet, A: Asset, B: Bundle> ProcGenExamplesPlugin<T, A, B> {
    pub fn new(generation_view_mode: GenerationViewMode) -> Self {
        Self {
            generation_view_mode,
            typestate: PhantomData,
        }
    }
}

impl<T: SharableDirectionSet, A: Asset, B: Bundle> Plugin for ProcGenExamplesPlugin<T, A, B> {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_spawning_nodes_scale);
        match self.generation_view_mode {
            GenerationViewMode::StepByStep(interval) => {
                app.add_systems(Update, step_by_step_timed_update::<T, A, B>);
                app.insert_resource(GenerationTimer(Timer::new(
                    Duration::from_millis(interval),
                    TimerMode::Repeating,
                )));
            }
            GenerationViewMode::StepByStepPaused => {
                app.add_systems(Update, step_by_step_input_update::<T, A, B>);
            }
            GenerationViewMode::Final => {
                app.add_systems(PostStartup, generate_all::<T, A, B>);
            }
        }
    }
}

pub fn generate_all<T: SharableDirectionSet, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut generation: ResMut<Generation<T, A, B>>,
) {
    let output = generation.gen.generate().unwrap();
    for (node_index, node) in output.nodes().iter().enumerate() {
        spawn_node(&mut commands, &mut generation, node, node_index);
    }
}

pub fn step_by_step_input_update<T: SharableDirectionSet, A: Asset, B: Bundle>(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    mut generation: ResMut<Generation<T, A, B>>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    if keys.pressed(KeyCode::Space) || buttons.just_pressed(MouseButton::Left) {
        step(&mut commands, &mut generation, existing_nodes);
    }
}

pub fn step_by_step_timed_update<T: SharableDirectionSet, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut generation: ResMut<Generation<T, A, B>>,
    mut timer: ResMut<GenerationTimer>,
    time: Res<Time>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        step(&mut commands, &mut generation, existing_nodes);
    }
}

fn step<T: SharableDirectionSet, A: Asset, B: Bundle>(
    commands: &mut Commands,
    generation: &mut ResMut<Generation<T, A, B>>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    let mut despawn_nodes = false;
    loop {
        let (generation_reset, nodes_to_spawn) = select_propagate_and_observe(generation);
        if generation_reset {
            despawn_nodes = true;
        }
        let mut non_void_spawned = false;
        for (node_index, generated_node) in nodes_to_spawn {
            if spawn_node(commands, generation, &generated_node, node_index) {
                non_void_spawned = true;
            }
        }
        if non_void_spawned {
            break;
        }
    }

    if despawn_nodes {
        for existing_node in existing_nodes.iter() {
            commands.entity(existing_node).despawn_recursive();
        }
    }
}

fn select_propagate_and_observe<T: SharableDirectionSet, A: Asset, B: Bundle>(
    generation: &mut ResMut<Generation<T, A, B>>,
) -> (bool, Vec<(usize, GeneratedNode)>) {
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

    (despawn_nodes, nodes_to_spawn)
}

/// Returns true if an entity was spawned. Some nodes are void and don't spawn any entity.
pub fn spawn_node<T: SharableDirectionSet, A: Asset, B: Bundle>(
    commands: &mut Commands,
    generation: &mut ResMut<Generation<T, A, B>>,
    node: &GeneratedNode,
    node_index: usize,
) -> bool {
    if let Some(asset) = generation.models_assets.get(&node.model_index) {
        let pos = generation.gen.grid().get_position(node_index);
        // +0.5*scale to center the node because its center is at its origin
        let translation = Vec3::new(
            generation.node_scale.x * (pos.x as f32 + 0.5),
            generation.node_scale.y * (pos.y as f32 + 0.5),
            generation.node_scale.z * (pos.z as f32 + 0.5),
        );
        let node_entity = commands
            .spawn((
                (generation.bundle_spawner)(
                    asset.clone(),
                    translation,
                    generation.assets_initial_scale,
                    f32::to_radians(node.rotation.value() as f32),
                ),
                SpawnedNode,
            ))
            .id();
        if let Some(animation) = &generation.spawn_animation {
            commands.entity(node_entity).insert(animation.clone());
        }
        commands
            .entity(generation.grid_entity)
            .add_child(node_entity);
        true
    } else {
        false
    }
}

/// Uses the z+ axis as the rotation axis
pub fn sprite_node_spawner(
    texture: Handle<Image>,
    translation: Vec3,
    scale: Vec3,
    rot_rad: f32,
) -> SpriteBundle {
    SpriteBundle {
        texture,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_z(rot_rad)),
        ..default()
    }
}

/// Uses the y+ axis as the rotation axis
pub fn scene_node_spawner(
    scene: Handle<Scene>,
    translation: Vec3,
    scale: Vec3,
    rot_rad: f32,
) -> SceneBundle {
    SceneBundle {
        scene,
        transform: Transform::from_translation(translation)
            .with_scale(scale)
            .with_rotation(Quat::from_rotation_y(rot_rad)),
        ..default()
    }
}
