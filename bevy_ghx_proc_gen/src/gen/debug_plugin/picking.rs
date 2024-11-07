use bevy::{
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::{Added, Changed, With, Without},
        system::{Commands, Local, Query, Res, ResMut, Resource},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt, Parent},
    input::{keyboard::KeyCode, ButtonInput},
    math::{primitives::Cuboid, Vec2, Vec3},
    pbr::{AlphaMode, NotShadowCaster, PbrBundle, StandardMaterial},
    prelude::{Deref, DerefMut},
    render::{color::Color, mesh::Mesh},
    sprite::{Sprite, SpriteBundle},
    text::Text,
    transform::components::Transform,
    utils::default,
};

use bevy_ghx_grid::{
    debug_plugin::{
        get_translation_from_grid_coords_3d,
        markers::{GridMarker, MarkerDespawnEvent},
        view::{DebugGridView, DebugGridView2d, DebugGridView3d},
    },
    ghx_grid::{coordinate_system::CoordinateSystem, direction::Direction},
};
use bevy_mod_picking::{
    events::Out,
    prelude::{Down, ListenerInput, On, Over, Pointer},
    PickableBundle,
};
use ghx_proc_gen::{
    generator::Generator,
    ghx_grid::{
        cartesian::{coordinates::CartesianCoordinates, grid::CartesianGrid},
        grid::Grid,
    },
    NodeIndex,
};

use crate::gen::GridNode;

use super::{
    cursor::{
        cursor_info_to_string, Cursor, CursorBehavior, CursorInfo, CursorMarkerSettings,
        CursorsPanelText, SelectCursor, TargetedNode, OVER_CURSOR_SECTION_INDEX,
    },
    generation::{ActiveGeneration, GenerationEvent},
    ProcGenKeyBindings,
};

/// Used to customize the color of the Over cursor [GridMarker]
#[derive(Resource)]
pub struct OverCursorMarkerSettings(pub Color);
impl Default for OverCursorMarkerSettings {
    fn default() -> Self {
        Self(Color::rgb(0.85, 0.85, 0.73))
    }
}
impl CursorMarkerSettings for OverCursorMarkerSettings {
    fn color(&self) -> Color {
        self.0
    }
}

/// Main component for the Over cursor
#[derive(Component, Debug)]
pub struct OverCursor;
impl CursorBehavior for OverCursor {
    fn new() -> Self {
        Self
    }
    fn updates_active_gen() -> bool {
        false
    }
}

/// Event raised when a node starts being overed by a mouse pointer
#[derive(Event, Deref, DerefMut)]
pub struct NodeOverEvent(pub Entity);
impl From<ListenerInput<Pointer<Over>>> for NodeOverEvent {
    fn from(event: ListenerInput<Pointer<Over>>) -> Self {
        NodeOverEvent(event.listener())
    }
}

/// Event raised when a node stops being overed by a mouse pointer
#[derive(Event, Deref, DerefMut)]
pub struct NodeOutEvent(pub Entity);
impl From<ListenerInput<Pointer<Out>>> for NodeOutEvent {
    fn from(event: ListenerInput<Pointer<Out>>) -> Self {
        NodeOutEvent(event.listener())
    }
}

/// Event raised when a node is selected by a mouse pointer
#[derive(Event, Deref, DerefMut)]
pub struct NodeSelectedEvent(pub Entity);

/// System that inserts picking event handlers to entites with an added [GridNode] component
pub fn insert_cursor_picking_handlers_to_grid_nodes<C: CoordinateSystem>(
    mut commands: Commands,
    spawned_nodes: Query<Entity, Added<GridNode>>,
) {
    use bevy_mod_picking::{pointer::PointerButton, prelude::ListenerMut};

    for entity in spawned_nodes.iter() {
        commands.entity(entity).try_insert((
            On::<Pointer<Over>>::send_event::<NodeOverEvent>(),
            On::<Pointer<Out>>::send_event::<NodeOutEvent>(),
            On::<Pointer<Down>>::run(
                move |event: ListenerMut<Pointer<Down>>,
                      mut selection_events: EventWriter<NodeSelectedEvent>| {
                    if event.button == PointerButton::Primary {
                        selection_events.send(NodeSelectedEvent(event.listener()));
                    }
                },
            ),
        ));
    }
}

/// System that update the over cursor UI panel
pub fn update_over_cursor_panel_text(
    mut cursors_panel_text: Query<&mut Text, With<CursorsPanelText>>,
    updated_cursors: Query<(&CursorInfo, &Cursor), (Changed<CursorInfo>, With<OverCursor>)>,
) {
    if let Ok((cursor_info, cursor)) = updated_cursors.get_single() {
        for mut text in &mut cursors_panel_text {
            let ui_text = &mut text.sections[OVER_CURSOR_SECTION_INDEX].value;
            match &cursor.0 {
                Some(overed_node) => {
                    *ui_text = format!(
                        "Hovered:\n{}",
                        cursor_info_to_string(overed_node, cursor_info)
                    );
                }
                None => ui_text.clear(),
            }
        }
    }
}

/// System updating the Over [Cursor] by reading all the [GenerationEvent]
///
/// Should run after update_cursors_info_on_cursors_changes and before update_cursors_info_from_generation_events
pub fn update_over_cursor_from_generation_events<C: CoordinateSystem>(
    mut cursors_events: EventReader<GenerationEvent>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut over_cursor: Query<&mut Cursor, With<OverCursor>>,
) {
    let Ok(mut cursor) = over_cursor.get_single_mut() else {
        return;
    };
    for event in cursors_events.read() {
        match event {
            GenerationEvent::Reinitialized(_grid_entity) => {
                // If there is an Over cursor, force despawn it, since we will despawn the underlying node there won't be any NodeOutEvent.
                if let Some(overed_node) = &cursor.0 {
                    marker_events.send(MarkerDespawnEvent::Marker(overed_node.marker));
                    cursor.0 = None;
                }
            }
            GenerationEvent::Updated(_grid_entity, _node_index) => {}
        }
    }
}

/// System used to update cursor positions from picking events
pub fn picking_update_cursors_position<
    C: CartesianCoordinates,
    CS: CursorMarkerSettings,
    CB: CursorBehavior,
    PE: Event + std::ops::DerefMut<Target = Entity>,
>(
    mut commands: Commands,
    cursor_marker_settings: Res<CS>,
    mut active_generation: ResMut<ActiveGeneration>,
    mut events: EventReader<PE>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    grid_nodes: Query<(&GridNode, &Parent)>,
    mut cursor: Query<&mut Cursor, With<CB>>,
    generations: Query<(Entity, &CartesianGrid<C>), With<Generator<C, CartesianGrid<C>>>>,
) {
    if let Some(event) = events.read().last() {
        let Ok(mut cursor) = cursor.get_single_mut() else {
            return;
        };
        let Ok((node, node_parent)) = grid_nodes.get(*event.deref()) else {
            return;
        };

        let picked_grid_entity = node_parent.get();
        let update_cursor = match &cursor.0 {
            Some(targeted_node) => {
                if (targeted_node.grid != picked_grid_entity)
                    || (targeted_node.node_index != node.0)
                {
                    marker_events.send(MarkerDespawnEvent::Marker(targeted_node.marker));
                    true
                } else {
                    false
                }
            }
            None => true,
        };

        if update_cursor {
            let Ok((gen_entity, grid)) = generations.get(picked_grid_entity) else {
                return;
            };

            if CB::updates_active_gen() {
                active_generation.0 = Some(gen_entity);
            }
            let position = grid.pos_from_index(node.0);
            let marker = commands
                .spawn(GridMarker::new(cursor_marker_settings.color(), position))
                .id();
            commands.entity(picked_grid_entity).add_child(marker);
            cursor.0 = Some(TargetedNode {
                grid: picked_grid_entity,
                node_index: node.0,
                position,
                marker,
            });
        }
    }
}

/// System used to remove an Over cursor on a [NodeOutEvent]
pub fn picking_remove_previous_over_cursor<C: CoordinateSystem>(
    mut out_events: EventReader<NodeOutEvent>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    mut nodes: Query<&GridNode>,
    mut over_cursor: Query<&mut Cursor, With<OverCursor>>,
) {
    if let Some(event) = out_events.read().last() {
        let Ok(mut cursor) = over_cursor.get_single_mut() else {
            return;
        };
        let Some(overed_node) = &cursor.0 else {
            return;
        };
        if let Ok(node) = nodes.get_mut(**event) {
            if overed_node.node_index == node.0 {
                marker_events.send(MarkerDespawnEvent::Marker(overed_node.marker));
                cursor.0 = None;
            }
        }
    }
}

/// Settings and assets used by the [CursorTarget]
#[derive(Resource, Default)]
pub struct CursorTargetAssets {
    color: Color,
    base_size: f32,
    target_mesh_3d: Handle<Mesh>,
    target_mat_3d: Handle<StandardMaterial>,
}

/// System used to insert default values into [CursorTargetAssets]
pub fn setup_picking_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_target_assets: ResMut<CursorTargetAssets>,
) {
    cursor_target_assets.color = Color::WHITE.with_a(0.15);
    cursor_target_assets.base_size = 0.9;
    cursor_target_assets.target_mesh_3d = meshes.add(Mesh::from(Cuboid {
        half_size: Vec3::splat(cursor_target_assets.base_size / 2.),
    }));
    cursor_target_assets.target_mat_3d = standard_materials.add(StandardMaterial {
        base_color: cursor_target_assets.color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
}

/// Main component marker for a cursor target
#[derive(Component)]
pub struct CursorTarget;

/// Local system resource used to cache and track cursor targets current siutation
#[derive(Default)]
pub struct ActiveCursorTargets {
    /// Current axis
    pub axis: Direction,
    /// Current source node
    pub from_node: NodeIndex,
}

/// System that spawn & depsanw the cursor targets
pub fn update_cursor_targets_nodes<C: CartesianCoordinates>(
    mut local_active_cursor_targets: Local<Option<ActiveCursorTargets>>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    cursor_target_assets: Res<CursorTargetAssets>,
    proc_gen_key_bindings: Res<ProcGenKeyBindings>,
    mut marker_events: EventWriter<MarkerDespawnEvent>,
    selection_cursor: Query<&Cursor, With<SelectCursor>>,
    mut over_cursor: Query<&mut Cursor, (With<OverCursor>, Without<SelectCursor>)>,
    grids_with_cam3d: Query<(&CartesianGrid<C>, &DebugGridView), With<DebugGridView3d>>,
    grids_with_cam2d: Query<
        (&CartesianGrid<C>, &DebugGridView),
        (With<DebugGridView2d>, Without<DebugGridView3d>),
    >,
    cursor_targets: Query<Entity, With<CursorTarget>>,
) {
    let Ok(selection_cursor) = selection_cursor.get_single() else {
        return;
    };
    let Some(selected_node) = &selection_cursor.0 else {
        return;
    };

    let axis_selection = if keys.pressed(proc_gen_key_bindings.cursor_x_axis) {
        Some(Direction::XForward)
    } else if keys.pressed(proc_gen_key_bindings.cursor_y_axis) {
        Some(Direction::YForward)
    } else if keys.pressed(proc_gen_key_bindings.cursor_z_axis) {
        Some(Direction::ZForward)
    } else {
        None
    };

    if let Some(axis) = axis_selection {
        if let Some(active_targets) = local_active_cursor_targets.as_mut() {
            if selected_node.node_index != active_targets.from_node {
                despawn_cursor_targets(
                    &mut commands,
                    &mut marker_events,
                    &cursor_targets,
                    &mut over_cursor,
                );
                spawn_cursor_targets(
                    &mut commands,
                    &cursor_target_assets,
                    selected_node,
                    axis,
                    &grids_with_cam3d,
                    &grids_with_cam2d,
                );

                active_targets.from_node = selected_node.node_index;
                active_targets.axis = axis;
            }
        } else {
            spawn_cursor_targets(
                &mut commands,
                &cursor_target_assets,
                selected_node,
                axis,
                &grids_with_cam3d,
                &grids_with_cam2d,
            );

            *local_active_cursor_targets = Some(ActiveCursorTargets {
                axis,
                from_node: selected_node.node_index,
            });
        }
    } else if local_active_cursor_targets.is_some() {
        *local_active_cursor_targets = None;
        despawn_cursor_targets(
            &mut commands,
            &mut marker_events,
            &cursor_targets,
            &mut over_cursor,
        );
    }
}

/// Function used to despawn all cursor targets and eventually the attached over cursor
pub fn despawn_cursor_targets(
    commands: &mut Commands,
    marker_events: &mut EventWriter<MarkerDespawnEvent>,
    cursor_targets: &Query<Entity, With<CursorTarget>>,
    over_cursor: &mut Query<&mut Cursor, (With<OverCursor>, Without<SelectCursor>)>,
) {
    for cursor_target in cursor_targets.iter() {
        commands.entity(cursor_target).despawn_recursive();
    }
    if let Ok(mut over_cursor) = over_cursor.get_single_mut() {
        // If there is an Over cursor, force despawn it, since we will despawn the underlying node there won't be any NodeOutEvent.
        if let Some(grid_cursor) = &over_cursor.0 {
            marker_events.send(MarkerDespawnEvent::Marker(grid_cursor.marker));
            over_cursor.0 = None;
        }
    };
}

/// Function used to spawn cursor targets
pub fn spawn_cursor_targets<C: CartesianCoordinates>(
    commands: &mut Commands,
    cursor_target_assets: &Res<CursorTargetAssets>,
    selected_node: &TargetedNode,
    axis: Direction,
    grids_with_cam3d: &Query<(&CartesianGrid<C>, &DebugGridView), With<DebugGridView3d>>,
    grids_with_cam2d: &Query<
        (&CartesianGrid<C>, &DebugGridView),
        (With<DebugGridView2d>, Without<DebugGridView3d>),
    >,
) {
    if let Ok((grid, grid_view)) = grids_with_cam3d.get(selected_node.grid) {
        spawn_cursor_targets_3d(
            commands,
            &cursor_target_assets,
            axis,
            selected_node,
            grid,
            &grid_view.node_size,
        );
    } else if let Ok((grid, grid_view)) = grids_with_cam2d.get(selected_node.grid) {
        spawn_cursor_targets_2d(
            commands,
            &cursor_target_assets,
            axis,
            selected_node,
            grid,
            &grid_view.node_size,
        );
    }
}

/// Function used to spawn cursor targets when using a 3d camera
pub fn spawn_cursor_targets_3d<C: CartesianCoordinates>(
    commands: &mut Commands,
    cursor_target_assets: &Res<CursorTargetAssets>,
    axis: Direction,
    selected_node: &TargetedNode,
    grid: &CartesianGrid<C>,
    node_size: &Vec3,
) {
    let mut spawn_cursor_target = |x: u32, y: u32, z: u32| {
        let translation = get_translation_from_grid_coords_3d(x, y, z, node_size);
        let helper_node_entity = commands
            .spawn((
                GridNode(grid.index_from_coords(x, y, z)),
                CursorTarget,
                NotShadowCaster,
                PbrBundle {
                    transform: Transform::from_translation(translation).with_scale(*node_size),
                    mesh: cursor_target_assets.target_mesh_3d.clone(),
                    material: cursor_target_assets.target_mat_3d.clone(),
                    ..default()
                },
            ))
            .id();
        commands
            .entity(selected_node.grid)
            .add_child(helper_node_entity);
    };

    match axis {
        Direction::XForward => {
            for x in 0..grid.size_x() {
                spawn_cursor_target(x, selected_node.position.y, selected_node.position.z);
            }
            for y in 0..grid.size_y() {
                for z in 0..grid.size_z() {
                    spawn_cursor_target(selected_node.position.x, y, z);
                }
            }
        }
        Direction::YForward => {
            for y in 0..grid.size_y() {
                spawn_cursor_target(selected_node.position.x, y, selected_node.position.z);
            }
            for x in 0..grid.size_x() {
                for z in 0..grid.size_z() {
                    spawn_cursor_target(x, selected_node.position.y, z);
                }
            }
        }
        Direction::ZForward => {
            for z in 0..grid.size_z() {
                spawn_cursor_target(selected_node.position.x, selected_node.position.y, z);
            }
            for x in 0..grid.size_x() {
                for y in 0..grid.size_y() {
                    spawn_cursor_target(x, y, selected_node.position.z);
                }
            }
        }
        _ => {}
    }
}

/// Function used to spawn cursor targets when using a 2d camera
pub fn spawn_cursor_targets_2d<C: CartesianCoordinates>(
    commands: &mut Commands,
    cursor_target_assets: &Res<CursorTargetAssets>,
    axis: Direction,
    selected_node: &TargetedNode,
    grid: &CartesianGrid<C>,
    node_size: &Vec3,
) {
    let mut spawn_cursor_target = |x: u32, y: u32, z: u32| {
        let mut translation = get_translation_from_grid_coords_3d(x, y, z, node_size);
        translation.z += node_size.z;
        let helper_node_entity = commands
            .spawn((
                GridNode(grid.index_from_coords(x, y, z)),
                CursorTarget,
                // TODO: Here MaterialMesh2dBundle + PickableBundle::default() did not interact with picking. Not sure why yet. Using Sprite instead.
                SpriteBundle {
                    transform: Transform::from_translation(translation).with_scale(*node_size),
                    sprite: Sprite {
                        color: cursor_target_assets.color,
                        custom_size: Some(Vec2::splat(cursor_target_assets.base_size)),
                        ..default()
                    },
                    ..default()
                },
                PickableBundle::default(),
            ))
            .id();
        commands
            .entity(selected_node.grid)
            .add_child(helper_node_entity);
    };

    match axis {
        Direction::XForward | Direction::YForward => {
            for x in 0..grid.size_x() {
                spawn_cursor_target(x, selected_node.position.y, selected_node.position.z);
            }
            for y in 0..grid.size_y() {
                spawn_cursor_target(selected_node.position.x, y, selected_node.position.z);
            }
        }
        Direction::ZForward => {
            for x in 0..grid.size_x() {
                for y in 0..grid.size_y() {
                    spawn_cursor_target(x, y, selected_node.position.z);
                }
            }
        }
        _ => {}
    }
}
