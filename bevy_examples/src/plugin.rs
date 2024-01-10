use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, Startup, Update},
    asset::{Asset, Handle},
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        bundle::Bundle,
        entity::Entity,
        event::EventWriter,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    input::{keyboard::KeyCode, mouse::MouseButton, Input},
    log::{info, warn},
    math::{Quat, Vec3},
    render::{color::Color, texture::Image},
    scene::{Scene, SceneBundle},
    sprite::SpriteBundle,
    text::TextStyle,
    time::{Time, Timer, TimerMode},
    transform::components::Transform,
    ui::node_bundles::TextBundle,
    utils::default,
};
use bevy_ghx_proc_gen::{
    grid::{markers::MarkerEvent, GridDebugPlugin, SharableCoordSystem},
    proc_gen::{
        generator::{model::ModelInstance, observer::GenerationUpdate, GenerationStatus},
        GenerationError,
    },
};

use crate::{
    anim::animate_spawning_nodes_scale,
    fps::{fps_text_update_system, setup_fps_counter},
    utils::{toggle_debug_grid_visibility, toggle_fps_counter},
    Generation, GenerationControl, GenerationControlStatus, GenerationViewMode, SpawnedNode,
    StepByStepTimed,
};

pub struct ProcGenExamplesPlugin<T: SharableCoordSystem, A: Asset, B: Bundle> {
    generation_view_mode: GenerationViewMode,
    typestate: PhantomData<(T, A, B)>,
}

impl<T: SharableCoordSystem, A: Asset, B: Bundle> ProcGenExamplesPlugin<T, A, B> {
    pub fn new(generation_view_mode: GenerationViewMode) -> Self {
        Self {
            generation_view_mode,
            typestate: PhantomData,
        }
    }
}

impl<D: SharableCoordSystem, A: Asset, B: Bundle> Plugin for ProcGenExamplesPlugin<D, A, B> {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            GridDebugPlugin::<D>::new(),
        ));
        app.insert_resource(self.generation_view_mode);
        app.add_systems(Startup, setup_ui);
        app.add_systems(
            Update,
            (
                animate_spawning_nodes_scale,
                update_generation_control,
                toggle_debug_grid_visibility,
            ),
        );

        // Fps
        app.add_systems(Startup, setup_fps_counter)
            .add_systems(Update, (fps_text_update_system, toggle_fps_counter));

        match self.generation_view_mode {
            GenerationViewMode::StepByStepTimed(steps, interval) => {
                app.add_systems(
                    Update,
                    (
                        step_by_step_timed_update::<D, A, B>,
                        update_generation_view::<D, A, B>,
                    )
                        .chain(),
                );
                app.insert_resource(StepByStepTimed {
                    steps,
                    timer: Timer::new(Duration::from_millis(interval), TimerMode::Repeating),
                });
            }
            GenerationViewMode::StepByStepPaused => {
                app.add_systems(
                    Update,
                    (
                        step_by_step_input_update::<D, A, B>,
                        update_generation_view::<D, A, B>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (generate_all::<D, A, B>, update_generation_view::<D, A, B>).chain(),
                );
            }
        }
    }
}

pub fn setup_ui(mut commands: Commands, view_mode: Res<GenerationViewMode>) {
    let mut controls_text = "`F1` toggle grid | `F2` toggle fps display\n\
    `Space` new generation"
        .to_string();
    if *view_mode == GenerationViewMode::StepByStepPaused {
        controls_text.push_str(
            "\n\
        'Up' or 'Right' advance the generation",
        );
    }
    commands.spawn(TextBundle::from_section(
        controls_text,
        TextStyle {
            font_size: 14.,
            ..Default::default()
        },
    ));
}

pub fn generate_all<D: SharableCoordSystem, A: Asset, B: Bundle>(
    mut generation: ResMut<Generation<D, A, B>>,
    mut generation_control: ResMut<GenerationControl>,
) {
    if generation_control.status == GenerationControlStatus::Ongoing {
        match generation.gen.generate() {
            Ok(()) => {
                info!(
                    "Generation done, seed: {}; grid: {}",
                    generation.gen.get_seed(),
                    generation.gen.grid()
                );
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
        generation_control.status = GenerationControlStatus::Paused;
    }
}

pub fn update_generation_control(
    keys: Res<Input<KeyCode>>,
    mut generation_control: ResMut<GenerationControl>,
) {
    if keys.just_pressed(KeyCode::Space) {
        match generation_control.status {
            GenerationControlStatus::Paused => {
                generation_control.status = GenerationControlStatus::Ongoing;
            }
            GenerationControlStatus::Ongoing => (),
        }
    }
}

pub fn step_by_step_input_update<D: SharableCoordSystem, A: Asset, B: Bundle>(
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    mut generation: ResMut<Generation<D, A, B>>,
    mut generation_control: ResMut<GenerationControl>,
) {
    if generation_control.status == GenerationControlStatus::Ongoing
        && (keys.just_pressed(KeyCode::Right)
            || keys.pressed(KeyCode::Up)
            || buttons.just_pressed(MouseButton::Left))
    {
        step_generation(&mut generation, &mut generation_control);
    }
}

pub fn step_by_step_timed_update<D: SharableCoordSystem, A: Asset, B: Bundle>(
    mut generation: ResMut<Generation<D, A, B>>,
    mut generation_control: ResMut<GenerationControl>,
    mut steps_and_timer: ResMut<StepByStepTimed>,
    time: Res<Time>,
) {
    steps_and_timer.timer.tick(time.delta());
    if steps_and_timer.timer.finished()
        && generation_control.status == GenerationControlStatus::Ongoing
    {
        for _ in 0..steps_and_timer.steps {
            step_generation(&mut generation, &mut generation_control);
            if generation_control.status != GenerationControlStatus::Ongoing {
                break;
            }
        }
    }
}

fn update_generation_view<D: SharableCoordSystem, A: Asset, B: Bundle>(
    mut commands: Commands,
    mut generation: ResMut<Generation<D, A, B>>,
    mut marker_events: EventWriter<MarkerEvent>,
    existing_nodes: Query<Entity, With<SpawnedNode>>,
) {
    let mut reinitialized = false;
    let mut nodes_to_spawn = Vec::new();
    for update in generation.observer.dequeue_all() {
        match update {
            GenerationUpdate::Generated(grid_node) => {
                nodes_to_spawn.push(grid_node);
            }
            GenerationUpdate::Reinitializing(_) => {
                reinitialized = true;
                nodes_to_spawn.clear();
            }
            GenerationUpdate::Failed(node_index) => {
                marker_events.send(MarkerEvent::Add {
                    color: Color::RED,
                    grid_entity: generation.grid_entity,
                    node_index,
                });
            }
        }
    }

    if reinitialized {
        for existing_node in existing_nodes.iter() {
            commands.entity(existing_node).despawn_recursive();
        }
        marker_events.send(MarkerEvent::ClearAll);
    }

    for grid_node in nodes_to_spawn {
        spawn_node(
            &mut commands,
            &generation,
            &grid_node.model_instance,
            grid_node.node_index,
        );
    }
}

fn step_generation<D: SharableCoordSystem, A: Asset, B: Bundle>(
    generation: &mut ResMut<Generation<D, A, B>>,
    generation_control: &mut ResMut<GenerationControl>,
) {
    loop {
        let mut non_void_spawned = false;
        match generation.gen.select_and_propagate_collected() {
            Ok((status, nodes_to_spawn)) => {
                for grid_node in nodes_to_spawn {
                    // We still collect the generated nodes here even though we don't really use them to spawn entities. We just check them for void nodes (for visualization purposes)
                    if let Some(assets) = generation
                        .models_assets
                        .get(&grid_node.model_instance.model_index)
                    {
                        if !assets.is_empty() {
                            non_void_spawned = true;
                        }
                    }
                }
                match status {
                    GenerationStatus::Ongoing => {}
                    GenerationStatus::Done => {
                        info!(
                            "Generation done, seed: {}; grid: {}",
                            generation.gen.get_seed(),
                            generation.gen.grid()
                        );
                        if generation_control.pause_when_done {
                            generation_control.status = GenerationControlStatus::Paused;
                        }
                        break;
                    }
                }
            }
            Err(GenerationError { node_index }) => {
                warn!(
                    "Generation Failed at node {}, seed: {}; grid: {}",
                    node_index,
                    generation.gen.get_seed(),
                    generation.gen.grid()
                );
                if generation_control.pause_on_error {
                    generation_control.status = GenerationControlStatus::Paused;
                }
                break;
            }
        }

        // If we want to skip over void nodes, we eep looping until we spawn a non-void
        if non_void_spawned | !generation_control.skip_void_nodes {
            break;
        }
    }
}

pub fn spawn_node<D: SharableCoordSystem, A: Asset, B: Bundle>(
    commands: &mut Commands,
    generation: &ResMut<Generation<D, A, B>>,
    instance: &ModelInstance,
    node_index: usize,
) {
    let empy = vec![];
    let node_assets = generation
        .models_assets
        .get(&instance.model_index)
        .unwrap_or(&empy);
    if node_assets.is_empty() {
        return;
    }

    let pos = generation.gen.grid().get_position(node_index);
    for node_asset in node_assets {
        let offset = node_asset.offset();
        // +0.5*scale to center the node because its center is at its origin
        let mut translation = Vec3::new(
            generation.node_scale.x * (pos.x as f32 + offset.dx as f32 + 0.5),
            generation.node_scale.y * (pos.y as f32 + offset.dy as f32 + 0.5),
            generation.node_scale.z * (pos.z as f32 + offset.dz as f32 + 0.5),
        );

        if generation.z_offset_from_y {
            translation.z += generation.node_scale.z
                * (1. - pos.y as f32 / generation.gen.grid().size_y() as f32);
        }

        let node_entity = commands
            .spawn((
                (generation.bundle_spawner)(
                    node_asset.handle.clone(),
                    translation,
                    generation.initial_assets_scale,
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
