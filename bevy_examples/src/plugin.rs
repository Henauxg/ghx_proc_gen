use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Startup, Update},
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        component::Component,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    gizmos::GizmoConfig,
    hierarchy::BuildChildren,
    input::{common_conditions::input_just_pressed, keyboard::KeyCode},
    math::Vec3,
    prelude::default,
    render::color::Color,
    text::{Text, TextSection, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        PositionType, Style, Val,
    },
};
use bevy_ghx_proc_gen::{
    gen::{
        assets::{AssetsBundleSpawner, ComponentSpawner, NoComponents},
        debug_plugin::{
            cursor::{CursorsOverlaysRoot, CursorsPanelRoot},
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

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> Plugin
    for ProcGenExamplesPlugin<C, A, T>
{
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            FpsDisplayPlugin,
            GridDebugPlugin::<C>::new(),
            DefaultPickingPlugins,
            ProcGenDebugPlugin::<C, A, T>::new(self.generation_view_mode, CursorUiMode::Overlay),
        ));
        app.insert_resource(SpawningScaleAnimation::new(
            0.8,
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
            ),
        );
    }
}

pub fn setup_gizmos_config(mut config: ResMut<GizmoConfig>) {
    config.depth_bias = -1.0;
}

pub const DEFAULT_EXAMPLES_FONT_SIZE: f32 = 16.;

/// Marker to find the container entity so we can show/hide the UI node
#[derive(Component)]
pub struct ExamplesUiRoot;

#[derive(Component)]
pub struct GenerationControltext;

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
    let mut keybindings_text =
        "Toggles: `F1` ui | `F2` fps | `F3` grid | `F4` markers | `F5` camera rotation\n\
       Selection: 'Esc' deselect | `Click` or `x/y/z`+`Left/Right` move selection | 'Tab' (switch active grid)\n"
            .to_string();

    if *view_mode == GenerationViewMode::StepByStepPaused {
        keybindings_text
            .push_str("Generation: 'Down' generate 1 step | 'Up' generates while pressed ");
    }
    let keybindings_ui = commands
        .spawn((
            Pickable::IGNORE,
            TextBundle {
                style: Style {
                    position_type: PositionType::Relative,
                    top: Val::Percent(1.),
                    ..default()
                },
                text: Text::from_sections([TextSection::new(
                    keybindings_text,
                    TextStyle {
                        font_size: DEFAULT_EXAMPLES_FONT_SIZE,
                        ..Default::default()
                    },
                )]),
                ..default()
            },
        ))
        .id();
    let status_ui = commands
        .spawn((
            Pickable::IGNORE,
            GenerationControltext,
            TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Percent(1.),
                    ..default()
                },
                text: Text::from_sections([
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
                ]),
                ..default()
            },
        ))
        .id();
    commands.entity(ui_root).add_child(keybindings_ui);
    commands.entity(ui_root).add_child(status_ui);
}

pub const GENERATION_CONTROL_STATUS_TEXT_SECTION_ID: usize = 1;
pub const GENERATION_CONTROLTEXT_SECTION_ID: usize = 2;

pub fn update_generation_control_ui(
    generation_control: Res<GenerationControl>,
    mut query: Query<&mut Text, With<GenerationControltext>>,
) {
    for mut text in &mut query {
        let control_section = &mut text.sections[GENERATION_CONTROLTEXT_SECTION_ID];
        control_section.value = format!(
            "\nskip_void_nodes: {}, pause_when_done: {}, pause_on_error: {}",
            generation_control.skip_void_nodes,
            generation_control.pause_when_done,
            generation_control.pause_on_error
        );

        let status_section = &mut text.sections[GENERATION_CONTROL_STATUS_TEXT_SECTION_ID];
        (status_section.value, status_section.style.color) = match generation_control.status {
            GenerationControlStatus::Ongoing => ("Ongoing".into(), Color::GREEN),
            GenerationControlStatus::Paused => {
                ("Paused ('Space' to unpause)".into(), Color::YELLOW_GREEN)
            }
        };
    }
}
