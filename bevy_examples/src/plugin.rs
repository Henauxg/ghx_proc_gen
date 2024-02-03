use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Startup, Update},
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        component::Component,
        schedule::IntoSystemConfigs,
        system::{Commands, Res, ResMut},
    },
    gizmos::GizmoConfig,
    hierarchy::BuildChildren,
    input::{common_conditions::input_just_pressed, keyboard::KeyCode},
    math::Vec3,
    text::TextStyle,
    ui::node_bundles::{NodeBundle, TextBundle},
};
use bevy_ghx_proc_gen::{
    gen::{
        assets::{AssetsBundleSpawner, ComponentSpawner, NoComponents},
        debug_plugin::{
            cursor::CursorsPanelRoot, CursorUiMode, GenerationViewMode, ProcGenDebugPlugin,
        },
        insert_bundle_from_resource_to_spawned_nodes,
    },
    grid::{toggle_debug_grids_visibilities, toggle_grid_markers_visibilities, GridDebugPlugin},
    proc_gen::grid::direction::CoordinateSystem,
};
use bevy_mod_picking::DefaultPickingPlugins;

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
                toggle_visibility::<KeybindingsUiRoot>.run_if(input_just_pressed(KeyCode::F1)),
                toggle_visibility::<CursorsPanelRoot>.run_if(input_just_pressed(KeyCode::F1)),
                toggle_visibility::<FpsRoot>.run_if(input_just_pressed(KeyCode::F2)),
                toggle_debug_grids_visibilities.run_if(input_just_pressed(KeyCode::F3)),
                toggle_grid_markers_visibilities.run_if(input_just_pressed(KeyCode::F4)),
                toggle_auto_orbit.run_if(input_just_pressed(KeyCode::F5)),
            ),
        );
    }
}

pub fn setup_gizmos_config(mut config: ResMut<GizmoConfig>) {
    config.depth_bias = -1.0;
}

/// Marker to find the container entity so we can show/hide the UI node
#[derive(Component)]
pub struct KeybindingsUiRoot;

pub fn setup_ui(mut commands: Commands, view_mode: Res<GenerationViewMode>) {
    let root = commands
        .spawn((KeybindingsUiRoot, NodeBundle::default()))
        .id();
    let mut controls_text = " `F1` ui | `F2` fps | `F3` grid | `F4` cursors | `F5` camera rotation\n `Space` unpause\n `Click` or `x/y/z`+`Left/Right` move selection"
        .to_string();

    if *view_mode == GenerationViewMode::StepByStepPaused {
        controls_text.push_str("\n 'Up' or 'Right' step the generation");
    }
    let text_ui = commands
        .spawn(TextBundle::from_section(
            controls_text,
            TextStyle {
                font_size: 16.,
                ..Default::default()
            },
        ))
        .id();
    commands.entity(root).add_child(text_ui);
}
