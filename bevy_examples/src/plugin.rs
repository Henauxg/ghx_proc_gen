use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, Update},
    asset::{Asset, Handle},
    ecs::{
        bundle::Bundle,
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    log::{error, info},
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
    proc_gen::generator::{node::ModelInstance, GenerationStatus},
};

use crate::{
    anim::animate_spawning_nodes_scale, Generation, GenerationControl, GenerationControlStatus,
    GenerationTimer, GenerationViewMode, SpawnedNode,
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
        app.add_systems(
            Update,
            (
                animate_spawning_nodes_scale,
                update_generation_control_status.before(clear_nodes),
                clear_nodes,
            ),
        )
        .add_event::<ClearNodeEvent>();

        match self.generation_view_mode {
            GenerationViewMode::StepByStep(interval) => {
                app.add_systems(
                    Update,
                    step_by_step_timed_update::<T, A, B>.before(clear_nodes),
                );
                app.insert_resource(GenerationTimer(Timer::new(
                    Duration::from_millis(interval),
                    TimerMode::Repeating,
                )));
            }
            GenerationViewMode::StepByStepPaused => {
                app.add_systems(
                    Update,
                    step_by_step_input_update::<T, A, B>.before(clear_nodes),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(Update, generate_all::<T, A, B>);
            }
        }
    }
}

pub fn generate_all<T: SharableDirectionSet, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut generation: ResMut<Generation<T, A, B>>,
    mut generation_control: ResMut<GenerationControl>,
) {
    if generation_control.status == GenerationControlStatus::Ongoing {
        match generation.gen.generate_collected() {
            Ok(output) => {
                for (node_index, node) in output.nodes().iter().enumerate() {
                    spawn_node(&mut commands, &mut generation, node, node_index);
                }
            }
            Err(_) => {
                error!("Failed to generate");
            }
        }
        generation_control.status = GenerationControlStatus::PausedNeedClear;
    }
}

pub fn update_generation_control_status(
    keys: Res<Input<KeyCode>>,
    mut generation_control: ResMut<GenerationControl>,
    mut clear_events: EventWriter<ClearNodeEvent>,
) {
    if keys.pressed(KeyCode::Space) {
        match generation_control.status {
            GenerationControlStatus::PausedNeedClear => {
                clear_events.send(ClearNodeEvent);
                generation_control.status = GenerationControlStatus::Ongoing;
            }
            GenerationControlStatus::Ongoing => (),
        }
    }
}

pub fn step_by_step_input_update<T: SharableDirectionSet, A: Asset, B: Bundle>(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    mut generation: ResMut<Generation<T, A, B>>,
    generation_control: ResMut<GenerationControl>,
    clear_events: EventWriter<ClearNodeEvent>,
) {
    if generation_control.status == GenerationControlStatus::Ongoing
        && (keys.just_pressed(KeyCode::Right)
            || keys.pressed(KeyCode::Up)
            || buttons.just_pressed(MouseButton::Left))
    {
        step_generation(
            &mut commands,
            &mut generation,
            clear_events,
            generation_control,
        );
    }
}

pub fn step_by_step_timed_update<T: SharableDirectionSet, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut generation: ResMut<Generation<T, A, B>>,
    generation_control: ResMut<GenerationControl>,
    mut timer: ResMut<GenerationTimer>,
    time: Res<Time>,
    clear_events: EventWriter<ClearNodeEvent>,
) {
    timer.0.tick(time.delta());
    if timer.0.finished() && generation_control.status == GenerationControlStatus::Ongoing {
        step_generation(
            &mut commands,
            &mut generation,
            clear_events,
            generation_control,
        );
    }
}

fn step_generation<T: SharableDirectionSet, A: Asset, B: Bundle>(
    commands: &mut Commands,
    generation: &mut ResMut<Generation<T, A, B>>,
    mut clear_events: EventWriter<ClearNodeEvent>,
    mut generation_control: ResMut<GenerationControl>,
) {
    loop {
        let mut non_void_spawned = false;
        match generation.gen.select_and_propagate_collected() {
            Ok((status, nodes_to_spawn)) => {
                for grid_node in nodes_to_spawn {
                    if spawn_node(
                        commands,
                        generation,
                        &grid_node.model_instance,
                        grid_node.node_index,
                    ) {
                        non_void_spawned = true;
                    }
                }
                match status {
                    GenerationStatus::Ongoing => {}
                    GenerationStatus::Done => {
                        info!("Generation done");
                        if generation_control.pause_when_done {
                            generation_control.status = GenerationControlStatus::PausedNeedClear;
                        } else {
                            clear_events.send(ClearNodeEvent);
                        }
                        break;
                    }
                }
            }

            Err(_) => {
                info!("Generation Failed");
                if generation_control.pause_on_error {
                    generation_control.status = GenerationControlStatus::PausedNeedClear;
                } else {
                    clear_events.send(ClearNodeEvent);
                }
                break;
            }
        }

        // Keep looping until we spawn a non-void, or if we want to skip over void nodes.
        if non_void_spawned | !generation_control.skip_void_nodes {
            break;
        }
    }
}

#[derive(Event)]
pub struct ClearNodeEvent;

fn clear_nodes(
    mut commands: Commands,
    mut clear_events: EventReader<ClearNodeEvent>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    if !clear_events.is_empty() {
        clear_events.clear();
        for existing_node in existing_nodes.iter() {
            commands.entity(existing_node).despawn_recursive();
        }
    }
}

/// Returns true if an entity was spawned. Some nodes are void and don't spawn any entity.
pub fn spawn_node<T: SharableDirectionSet, A: Asset, B: Bundle>(
    commands: &mut Commands,
    generation: &mut ResMut<Generation<T, A, B>>,
    instance: &ModelInstance,
    node_index: usize,
) -> bool {
    if let Some(asset) = generation.models_assets.get(&instance.model_index) {
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
                    generation.assets_scale,
                    f32::to_radians(instance.rotation.value() as f32),
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
