use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, Startup, Update},
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::{
        bundle::Bundle,
        system::{Commands, Res},
    },
    math::Vec3,
    text::TextStyle,
    ui::node_bundles::TextBundle,
};
use bevy_ghx_proc_gen::{
    gen::{
        debug_plugin::{GenerationViewMode, ProcGenDebugPlugin},
        insert_bundle_from_resource_to_spawned_nodes, AssetHandles,
    },
    grid::{GridDebugPlugin, SharableCoordSystem},
};

use crate::{
    anim::{animate_scale, ease_in_cubic, SpawningScaleAnimation},
    fps::FpsDisplayPlugin,
    utils::{toggle_debug_grids_visibilities, toggle_fps_counter},
};

pub struct ProcGenExamplesPlugin<C: SharableCoordSystem, A: AssetHandles, B: Bundle> {
    generation_view_mode: GenerationViewMode,
    assets_scale: Vec3,
    typestate: PhantomData<(C, A, B)>,
}

impl<C: SharableCoordSystem, A: AssetHandles, B: Bundle> ProcGenExamplesPlugin<C, A, B> {
    pub fn new(generation_view_mode: GenerationViewMode, assets_scale: Vec3) -> Self {
        Self {
            generation_view_mode,
            assets_scale,
            typestate: PhantomData,
        }
    }
}

impl<C: SharableCoordSystem, A: AssetHandles, B: Bundle> Plugin for ProcGenExamplesPlugin<C, A, B> {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            FpsDisplayPlugin,
            GridDebugPlugin::<C>::new(),
            ProcGenDebugPlugin::<C, A, B>::new(self.generation_view_mode),
        ));
        app.insert_resource(SpawningScaleAnimation::new(
            0.8,
            self.assets_scale,
            ease_in_cubic,
        ));
        app.add_systems(Startup, setup_ui);
        app.add_systems(
            Update,
            (
                insert_bundle_from_resource_to_spawned_nodes::<SpawningScaleAnimation>,
                animate_scale,
                toggle_debug_grids_visibilities,
                toggle_fps_counter,
            ),
        );
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
