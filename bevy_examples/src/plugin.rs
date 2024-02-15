use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PreUpdate, Startup, Update},
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        component::Component,
        event::Events,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    gizmos::GizmoConfig,
    hierarchy::BuildChildren,
    input::{
        common_conditions::input_just_pressed,
        keyboard::KeyCode,
        mouse::{MouseButton, MouseWheel},
        Input,
    },
    math::Vec3,
    prelude::default,
    render::color::Color,
    text::{BreakLineOn, Text, TextSection, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        PositionType, Style, UiRect, Val,
    },
};
use bevy_ghx_proc_gen::{
    bevy_egui::{self, EguiPlugin},
    gen::{
        assets::{AssetsBundleSpawner, ComponentSpawner, NoComponents},
        debug_plugin::{
            cursor::{CursorsOverlaysRoot, CursorsPanelRoot},
            egui_editor::{paint, update_painting_state, EditorContext},
            CursorUiMode, GenerationControl, GenerationControlStatus, GenerationViewMode,
            ProcGenDebugPlugin,
        },
        insert_bundle_from_resource_to_spawned_nodes,
    },
    grid::{toggle_debug_grids_visibilities, toggle_grid_markers_visibilities, GridDebugPlugin},
    proc_gen::grid::direction::CoordinateSystem,
};
use bevy_mod_picking::{picking_core::Pickable, DefaultPickingPlugins};

use crate::{
    anim::{animate_scale, ease_in_cubic, SpawningScaleAnimation},
    camera::toggle_auto_orbit,
    fps::{FpsDisplayPlugin, FpsRoot},
    utils::toggle_visibility,
};

pub struct ProcGenExamplesPlugin<
    C: CoordinateSystem,
    A: AssetsBundleSpawner,
    T: ComponentSpawner = NoComponents,
> {
    generation_view_mode: GenerationViewMode,
    assets_scale: Vec3,
    typestate: PhantomData<(C, A, T)>,
}

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner>
    ProcGenExamplesPlugin<C, A, T>
{
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

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> Plugin
    for ProcGenExamplesPlugin<C, A, T>
{
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            FpsDisplayPlugin,
            GridDebugPlugin::<C>::new(),
            DefaultPickingPlugins,
            EguiPlugin,
            ProcGenDebugPlugin::<C, A, T>::new(self.generation_view_mode, CursorUiMode::Overlay),
        ));
        app.insert_resource(SpawningScaleAnimation::new(
            DEFAULT_SPAWN_ANIMATION_DURATION,
            self.assets_scale,
            ease_in_cubic,
        ));
        app.add_systems(Startup, (setup_gizmos_config, setup_ui));
        app.add_systems(
            Update,
            (
                insert_bundle_from_resource_to_spawned_nodes::<SpawningScaleAnimation>,
                animate_scale,
                (
                    toggle_visibility::<ExamplesUiRoot>,
                    toggle_visibility::<CursorsPanelRoot>,
                    toggle_visibility::<CursorsOverlaysRoot>,
                )
                    .run_if(input_just_pressed(KeyCode::F1)),
                toggle_visibility::<FpsRoot>.run_if(input_just_pressed(KeyCode::F2)),
                toggle_debug_grids_visibilities.run_if(input_just_pressed(KeyCode::F3)),
                toggle_grid_markers_visibilities.run_if(input_just_pressed(KeyCode::F4)),
                toggle_auto_orbit.run_if(input_just_pressed(KeyCode::F5)),
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
                .after(bevy_egui::systems::process_input_system)
                .before(bevy_egui::EguiSet::BeginFrame),
        );
    }
}

pub fn setup_gizmos_config(mut config: ResMut<GizmoConfig>) {
    config.depth_bias = -1.0;
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

pub fn setup_ui(mut commands: Commands, view_mode: Res<GenerationViewMode>) {
    let ui_root = commands
        .spawn((
            ExamplesUiRoot,
            NodeBundle {
                style: Style {
                    left: Val::Percent(1.),
                    height: Val::Vh(100.),
                    ..default()
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut keybindings_text = "Toggles:\n\
        'F1' Show/hide UI\n\
        'F2' Show/hide fps\n\
        'F3' Show/hide grid\n\
        'F4' Show/hide markers\n\
        'F5' Enable/disable camera rotation\n\
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
            Pickable::IGNORE,
            NodeBundle {
                background_color: Color::BLACK.with_a(0.6).into(),
                style: Style {
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
                ..default()
            },
        ))
        .id();
    let keybindings_ui = commands
        .spawn((
            Pickable::IGNORE,
            TextBundle {
                style: Style {
                    position_type: PositionType::Relative,
                    ..default()
                },
                text: Text {
                    sections: vec![TextSection::new(
                        keybindings_text,
                        TextStyle {
                            font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                            ..Default::default()
                        },
                    )],
                    linebreak_behavior: BreakLineOn::NoWrap,
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    let status_ui = commands
        .spawn((
            Pickable::IGNORE,
            GenerationControlText,
            TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Percent(1.),
                    ..default()
                },
                text: Text {
                    sections: vec![
                        TextSection::new(
                            "\nGeneration control status: ",
                            TextStyle {
                                font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                                ..Default::default()
                            },
                        ),
                        TextSection::from_style(TextStyle {
                            font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                            ..Default::default()
                        }),
                        TextSection::from_style(TextStyle {
                            font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                            ..Default::default()
                        }),
                        TextSection::new(
                            format!("\nGenerationViewMode: {:?}", *view_mode),
                            TextStyle {
                                font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                                ..Default::default()
                            },
                        ),
                    ],
                    linebreak_behavior: BreakLineOn::NoWrap,
                    ..default()
                },
                ..default()
            },
        ))
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
    gen_control: Res<GenerationControl>,
    mut query: Query<&mut Text, With<GenerationControlText>>,
) {
    for mut text in &mut query {
        let status_section = &mut text.sections[GENERATION_CONTROL_STATUS_TEXT_SECTION_ID];
        (status_section.value, status_section.style.color) = match gen_control.status {
            GenerationControlStatus::Ongoing => ("Ongoing ('Space' to pause)".into(), Color::GREEN),
            GenerationControlStatus::Paused => {
                ("Paused ('Space' to unpause)".into(), Color::YELLOW_GREEN)
            }
        };

        let control_section = &mut text.sections[GENERATION_CONTROL_TEXT_SECTION_ID];
        control_section.value = format!(
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
    mut mouse: ResMut<Input<MouseButton>>,
    mut mouse_wheel: ResMut<Events<MouseWheel>>,
) {
    let ctx = contexts.ctx_mut();
    if ctx.wants_pointer_input() || ctx.is_pointer_over_area() {
        mouse.reset_all();
        mouse_wheel.clear();
    }
}
