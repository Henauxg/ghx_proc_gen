use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PreUpdate, Startup, Update},
    color::{
        palettes::css::{GREEN, YELLOW_GREEN},
        Alpha, Color,
    },
    ecs::{
        component::Component,
        event::Events,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    gizmos::config::GizmoConfigStore,
    hierarchy::BuildChildren,
    input::{
        common_conditions::input_just_pressed,
        keyboard::KeyCode,
        mouse::{MouseButton, MouseWheel},
        ButtonInput,
    },
    math::Vec3,
    prelude::{default, Entity, MeshPickingPlugin, PickingBehavior, Text, TextUiWriter},
    text::{LineBreak, TextFont, TextLayout, TextSpan},
    ui::{BackgroundColor, Node, PositionType, UiRect, Val},
};
use bevy_ghx_proc_gen::{
    assets::BundleInserter,
    bevy_egui::{self, EguiPlugin},
    bevy_ghx_grid::{
        debug_plugin::{
            markers::MarkersGroup, toggle_debug_grids_visibilities,
            toggle_grid_markers_visibilities, GridDebugPlugin,
        },
        ghx_grid::coordinate_system::CoordinateSystem,
    },
    debug_plugin::{
        cursor::{CursorsOverlaysRoot, CursorsPanelRoot},
        egui_editor::{paint, toggle_editor, update_painting_state, EditorContext},
        DebugPluginConfig, GenerationControl, GenerationControlStatus, GenerationViewMode,
        ProcGenDebugPlugins,
    },
    insert_bundle_from_resource_to_spawned_nodes,
    proc_gen::ghx_grid::cartesian::coordinates::CartesianCoordinates,
};
use bevy_ghx_utils::{camera::toggle_auto_orbit, systems::toggle_visibility};

use crate::anim::{animate_scale, ease_in_cubic, SpawningScaleAnimation};

pub struct ProcGenExamplesPlugin<C: CoordinateSystem, A: BundleInserter> {
    generation_view_mode: GenerationViewMode,
    assets_scale: Vec3,
    typestate: PhantomData<(C, A)>,
}

impl<C: CoordinateSystem, A: BundleInserter> ProcGenExamplesPlugin<C, A> {
    pub fn new(generation_view_mode: GenerationViewMode, assets_scale: Vec3) -> Self {
        Self {
            generation_view_mode,
            assets_scale,
            typestate: PhantomData,
        }
    }
}

const DEFAULT_SPAWN_ANIMATION_DURATION: f32 = 0.6;
const FAST_SPAWN_ANIMATION_DURATION: f32 = 0.1;

impl<C: CartesianCoordinates, A: BundleInserter> Plugin for ProcGenExamplesPlugin<C, A> {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MeshPickingPlugin,
            EguiPlugin,
            GridDebugPlugin::<C>::new(),
            ProcGenDebugPlugins::<C, A> {
                config: DebugPluginConfig {
                    generation_view_mode: self.generation_view_mode,
                    ..default()
                },
                ..default()
            },
        ));
        app.insert_resource(SpawningScaleAnimation::new(
            DEFAULT_SPAWN_ANIMATION_DURATION,
            self.assets_scale,
            ease_in_cubic,
        ));
        app.add_systems(
            Startup,
            (setup_examples_ui, customize_grid_markers_gizmos_config),
        );
        app.add_systems(
            Update,
            (
                insert_bundle_from_resource_to_spawned_nodes::<SpawningScaleAnimation>,
                animate_scale,
                (
                    toggle_visibility::<ExamplesUiRoot>,
                    toggle_visibility::<CursorsPanelRoot>,
                    toggle_visibility::<CursorsOverlaysRoot>,
                    toggle_editor,
                )
                    .run_if(input_just_pressed(KeyCode::F1)),
                toggle_debug_grids_visibilities.run_if(input_just_pressed(KeyCode::F2)),
                toggle_grid_markers_visibilities.run_if(input_just_pressed(KeyCode::F3)),
                toggle_auto_orbit.run_if(input_just_pressed(KeyCode::F4)),
                update_generation_control_ui,
                // Quick adjust of the slowish spawn animation to be more snappy when painting
                adjust_spawn_animation_when_painting
                    .after(update_painting_state)
                    .before(paint::<C>),
            ),
        );
        // Quick & dirty: silence bevy events when using an egui window
        app.add_systems(
            PreUpdate,
            absorb_egui_inputs
                .after(bevy_egui::input::write_egui_input_system)
                .before(bevy_egui::begin_pass_system),
        );
    }
}

pub fn customize_grid_markers_gizmos_config(mut config_store: ResMut<GizmoConfigStore>) {
    let markers_config = config_store.config_mut::<MarkersGroup>().0;
    // Make them appear on top of everything else
    markers_config.depth_bias = -1.0;
}

pub fn adjust_spawn_animation_when_painting(
    editor_contex: Res<EditorContext>,
    mut spawn_animation: ResMut<SpawningScaleAnimation>,
) {
    if editor_contex.painting {
        spawn_animation.duration_sec = FAST_SPAWN_ANIMATION_DURATION;
    } else {
        spawn_animation.duration_sec = DEFAULT_SPAWN_ANIMATION_DURATION;
    }
}

pub const DEFAULT_EXAMPLES_FONT_SIZE: f32 = 17.;

/// Marker to find the container entity so we can show/hide the UI node
#[derive(Component)]
pub struct ExamplesUiRoot;

#[derive(Component)]
pub struct GenerationControlText;

pub fn setup_examples_ui(mut commands: Commands, view_mode: Res<GenerationViewMode>) {
    let ui_root = commands
        .spawn((
            ExamplesUiRoot,
            Node {
                left: Val::Percent(1.),
                height: Val::Vh(100.),
                ..default()
            },
            PickingBehavior::IGNORE,
        ))
        .id();
    let mut keybindings_text = "Toggles:\n\
        'F1' Show/hide UI\n\
        'F2' Show/hide grid\n\
        'F3' Show/hide markers\n\
        'F4' Enable/disable camera rotation\n\
        \n\
        Selection:\n\
       'Click' Select\n\
       'x/y/z'+'Left/Right' Move selection\n\
       'Esc' Deselect\n\
       'Tab' Switch active grid\n"
        .to_string();

    if *view_mode == GenerationViewMode::StepByStepManual {
        keybindings_text.push_str(
            "\nGeneration:\n\
            'Down' Generate 1 step\n\
            'Up' Generate while pressed",
        );
    }
    let keybindings_ui_background = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(1.),
                padding: UiRect {
                    left: Val::Px(6.),
                    right: Val::Px(6.),
                    top: Val::Px(6.),
                    bottom: Val::Px(6.),
                },
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.6).into()),
            PickingBehavior::IGNORE,
        ))
        .id();
    let keybindings_ui = commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                ..default()
            },
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                ..default()
            },
            Text(keybindings_text),
            PickingBehavior::IGNORE,
        ))
        .id();
    let status_ui = commands
        .spawn((
            GenerationControlText,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Percent(1.),
                ..default()
            },
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextFont {
                font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                ..default()
            },
            PickingBehavior::IGNORE,
            Text("".into()),
        ))
        .with_child(TextSpan::new("\nGeneration control status: "))
        .with_child(TextSpan::new(""))
        .with_child(TextSpan::new(""))
        .with_child(TextSpan::new(format!(
            "\nGenerationViewMode: {:?}",
            *view_mode
        )))
        .id();
    commands
        .entity(ui_root)
        .add_child(keybindings_ui_background);
    commands
        .entity(keybindings_ui_background)
        .add_child(keybindings_ui);
    commands.entity(ui_root).add_child(status_ui);
}

pub const GENERATION_CONTROL_STATUS_TEXT_SECTION_ID: usize = 1;
pub const GENERATION_CONTROL_TEXT_SECTION_ID: usize = 2;
pub const GENERATION_VIEW_MODE_TEXT_SECTION_ID: usize = 3;

pub fn update_generation_control_ui(
    mut writer: TextUiWriter,
    gen_control: Res<GenerationControl>,
    mut query: Query<Entity, With<GenerationControlText>>,
) {
    for text_entity in &mut query {
        let (text, color) = match gen_control.status {
            GenerationControlStatus::Ongoing => ("Ongoing ('Space' to pause)".into(), GREEN.into()),
            GenerationControlStatus::Paused => {
                ("Paused ('Space' to unpause)".into(), YELLOW_GREEN.into())
            }
        };
        *writer.text(text_entity, GENERATION_CONTROL_STATUS_TEXT_SECTION_ID) = text;
        *writer.color(text_entity, GENERATION_CONTROL_STATUS_TEXT_SECTION_ID) = color;

        * writer.text(text_entity, GENERATION_CONTROL_TEXT_SECTION_ID)=  format!(
            "\nGenerationControl: skip_void_nodes: {}, pause_when_done: {}, pause_on_error: {}, pause_on_reinitialize: {}",
            gen_control.skip_void_nodes,
            gen_control.pause_when_done,
            gen_control.pause_on_error,
            gen_control.pause_on_reinitialize
        );
    }
}

// Quick & dirty: silence bevy events when using an egui window
fn absorb_egui_inputs(
    mut contexts: bevy_egui::EguiContexts,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut mouse_wheel: ResMut<Events<MouseWheel>>,
) {
    let ctx = contexts.ctx_mut();
    if ctx.wants_pointer_input() || ctx.is_pointer_over_area() {
        mouse.reset_all();
        mouse_wheel.clear();
    }
}
