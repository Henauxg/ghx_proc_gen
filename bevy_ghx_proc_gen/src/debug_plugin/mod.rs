use std::marker::PhantomData;

use bevy::{
    app::{App, Plugin, PluginGroup, PluginGroupBuilder},
    color::{Alpha, Color},
    ecs::system::Resource,
    input::keyboard::KeyCode,
};
use bevy_ghx_grid::ghx_grid::coordinate_system::CoordinateSystem;
use cursor::ProcGenDebugCursorPlugin;

use generation::{GenerationViewMode, ProcGenDebugGenerationPlugin};
use ghx_proc_gen::ghx_grid::cartesian::coordinates::CartesianCoordinates;
use picking::ProcGenDebugPickingPlugin;

use crate::{assets::BundleInserter, spawner_plugin::ProcGenSpawnerPlugin};

/// Module with picking features, enabled with the `picking` feature
#[cfg(feature = "picking")]
pub mod picking;

/// Module providing a small egui editor, enabled with the `egui-edit` feature
#[cfg(feature = "egui-edit")]
pub mod egui_editor;

/// Module providing all the grid cursors features
pub mod cursor;
/// Module handling the generation fetaures of the debug_plugin
pub mod generation;

/// Used to configure how the cursors UI should be displayed
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CursorUiMode {
    /// No cursor UI display
    None,
    /// Display as a UI panel on the screen UI
    Panel,
    /// Display as a small overlay panel over the [cursor::Cursor]
    #[default]
    Overlay,
}

/// Resource used to customize cursors UI
#[derive(Resource, Debug)]
pub struct GridCursorsUiSettings {
    /// Font size in the UI panels/overlays
    pub font_size: f32,
    /// Background color of the UI panels/overlays
    pub background_color: Color,
    /// Text colors in the UI panels/overlays
    pub text_color: Color,
}

impl Default for GridCursorsUiSettings {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            background_color: Color::BLACK.with_alpha(0.45),
            text_color: Color::WHITE,
        }
    }
}

/// Configuration for a [ProcGenDebugRunnerPlugin]
#[derive(Default, Clone)]
pub struct DebugPluginConfig {
    /// Controls how the generation occurs.
    pub generation_view_mode: GenerationViewMode,
    /// Used to configure how the cursors UI should be displayed
    pub cursor_ui_mode: CursorUiMode,
}

/// A [`Plugin`] useful for debug/analysis/demo. It mainly run [`ghx_proc_gen::generator::Generator`] components
///
/// It takes in a [`GenerationViewMode`] to control how the generators components will be run.
///
/// It also uses the following `Resources`: [`ProcGenKeyBindings`] and [`GenerationControl`] (and will init them to their defaults if not inserted by the user).
#[derive(Default)]
pub struct ProcGenDebugRunnerPlugin<C: CoordinateSystem> {
    /// Configuration of the debug plugin
    pub config: DebugPluginConfig,
    #[doc(hidden)]
    pub typestate: PhantomData<C>,
}

impl<C: CartesianCoordinates> Plugin for ProcGenDebugRunnerPlugin<C> {
    fn build(&self, app: &mut App) {
        // If the resources already exists, nothing happens, else, add them with default values.
        app.init_resource::<ProcGenKeyBindings>();

        app.add_plugins((
            ProcGenDebugGenerationPlugin::<C>::new(&self.config),
            ProcGenDebugCursorPlugin::<C>::new(&self.config),
        ));

        #[cfg(feature = "egui-edit")]
        app.add_plugins(egui_editor::plugin::<C>);

        #[cfg(feature = "picking")]
        app.add_plugins(ProcGenDebugPickingPlugin::<C>::new(&self.config));
    }
}

/// A group of plugins that combines debug generation and nodes spawning
#[derive(Default)]
pub struct ProcGenDebugPlugins<C: CartesianCoordinates, A: BundleInserter> {
    /// Configuration of the debug plugin
    pub config: DebugPluginConfig,
    #[doc(hidden)]
    pub typestate: PhantomData<(C, A)>,
}
impl<C: CartesianCoordinates, A: BundleInserter> PluginGroup for ProcGenDebugPlugins<C, A> {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ProcGenDebugRunnerPlugin::<C> {
                config: self.config,
                typestate: PhantomData,
            })
            .add(ProcGenSpawnerPlugin::<C, A>::new())
    }
}

/// Resource available to override the default keybindings used by the [`ProcGenDebugRunnerPlugin`], usign a QWERTY layout ()
#[derive(Resource)]
pub struct ProcGenKeyBindings {
    /// Key to move the selection cursor to the previous node on the current axis
    pub prev_node: KeyCode,
    /// Key to move the selection cursor to the next node on the current axis
    pub next_node: KeyCode,
    /// Key pressed to enable the X axis selection
    pub cursor_x_axis: KeyCode,
    /// Key pressed to enable the Y axis selection
    pub cursor_y_axis: KeyCode,
    /// Key pressed to enable the Z axis selection
    pub cursor_z_axis: KeyCode,
    /// Key to deselect the current selection
    pub deselect: KeyCode,
    /// Key to move the selection cursor to another grid
    pub switch_grid: KeyCode,

    /// Key to pause/unpause the current [`GenerationControlStatus`]
    pub pause_toggle: KeyCode,
    /// Key used only with [`GenerationViewMode::StepByStepManual`] to step once per press
    pub step: KeyCode,
    /// Key used only with [`GenerationViewMode::StepByStepManual`] to step continuously as long as pressed
    pub continuous_step: KeyCode,
}

impl Default for ProcGenKeyBindings {
    fn default() -> Self {
        Self {
            prev_node: KeyCode::ArrowLeft,
            next_node: KeyCode::ArrowRight,
            cursor_x_axis: KeyCode::KeyX,
            cursor_y_axis: KeyCode::KeyY,
            cursor_z_axis: KeyCode::KeyZ,
            deselect: KeyCode::Escape,
            switch_grid: KeyCode::Tab,
            pause_toggle: KeyCode::Space,
            step: KeyCode::ArrowDown,
            continuous_step: KeyCode::ArrowUp,
        }
    }
}
