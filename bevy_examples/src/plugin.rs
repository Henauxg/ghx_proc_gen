use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Startup, Update},
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        schedule::IntoSystemConfigs,
        system::{Commands, Res, ResMut},
    },
    gizmos::GizmoConfig,
    input::{common_conditions::input_just_pressed, keyboard::KeyCode},
    math::Vec3,
    text::TextStyle,
    ui::node_bundles::TextBundle,
};
use bevy_ghx_proc_gen::{
    bevy_mod_picking::DefaultPickingPlugins,
    gen::{
        assets::{AssetsBundleSpawner, ComponentSpawner, NoComponents},
        debug_plugin::{GenerationViewMode, ProcGenDebugPlugin},
        insert_bundle_from_resource_to_spawned_nodes,
    },
    grid::{toggle_debug_grids_visibilities, GridDebugPlugin},
    proc_gen::grid::direction::CoordinateSystem,
};

use crate::{
    anim::{animate_scale, ease_in_cubic, SpawningScaleAnimation},
    fps::{toggle_fps_counter, FpsDisplayPlugin},
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
            ProcGenDebugPlugin::<C, A, T>::new(self.generation_view_mode),
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
                toggle_debug_grids_visibilities.run_if(input_just_pressed(KeyCode::F1)),
                toggle_fps_counter.run_if(input_just_pressed(KeyCode::F2)),
            ),
        );
        app.add_systems(
            Update,
            toggle_debug_grids_visibilities.run_if(input_just_pressed(KeyCode::F1)),
        );
    }
}

pub fn setup_gizmos_config(mut config: ResMut<GizmoConfig>) {
    config.depth_bias = -1.0;
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
