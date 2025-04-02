use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{
        App, Plugin, PluginGroup, PluginGroupBuilder, PostStartup, PostUpdate, PreUpdate, Startup,
        Update,
    },
    color::{Alpha, Color},
    ecs::{schedule::IntoSystemConfigs, system::Resource},
    input::keyboard::KeyCode,
    time::{Timer, TimerMode},
};
use bevy_ghx_grid::ghx_grid::coordinate_system::CoordinateSystem;
use cursor::{update_cursors_info_on_generated_nodes, update_cursors_info_on_generation_reset};

use ghx_proc_gen::ghx_grid::cartesian::coordinates::CartesianCoordinates;
use picking::update_over_cursor_on_generation_reset;

use crate::{
    add_named_observer, assets::BundleInserter, spawner_plugin::ProcGenSpawnerPlugin,
    GenerationResetEvent, NodesGeneratedEvent,
};

use self::{
    cursor::{
        deselect_from_keybinds, move_selection_from_keybinds, setup_cursor, setup_cursors_overlays,
        setup_cursors_panel, switch_generation_selection_from_keybinds,
        update_cursors_info_on_cursors_changes, update_cursors_overlays,
        update_selection_cursor_panel_text, CursorKeyboardMovement, CursorKeyboardMovementSettings,
        SelectCursor, SelectionCursorMarkerSettings,
    },
    generation::{
        dequeue_generation_updates, generate_all, insert_error_markers_to_new_generations,
        step_by_step_input_update, step_by_step_timed_update, update_active_generation,
        update_generation_control, ActiveGeneration,
    },
};

#[cfg(feature = "picking")]
use self::picking::{
    insert_cursor_picking_handlers_on_grid_nodes, picking_remove_previous_over_cursor,
    picking_update_cursors_position, setup_picking_assets, update_cursor_targets_nodes,
    update_over_cursor_panel_text, CursorTargetAssets, NodeOutEvent, NodeOverEvent,
    NodeSelectedEvent, OverCursor, OverCursorMarkerSettings,
};

/// Module with picking features, enabled with the `picking` feature
#[cfg(feature = "picking")]
pub mod picking;

#[cfg(feature = "egui-edit")]
use self::egui_editor::{
    draw_edition_panel, editor_enabled, paint, update_brush, update_painting_state, BrushEvent,
    EditorConfig, EditorContext,
};

/// Module providing a small egui editor, enabled with the `egui-edit` feature
#[cfg(feature = "egui-edit")]
pub mod egui_editor;

/// Module providing all the grid cursors features
pub mod cursor;
/// Module handling the generation fetaures of the debug_plugin
pub mod generation;

/// Used to configure how the cursors UI should be displayed
#[derive(Default, Debug, PartialEq, Eq)]
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
#[derive(Default)]
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

impl<C: CartesianCoordinates> Plugin for ProcGenDebugRunnerPlugin<C> {
    // TODO Clean: Split into multiple plugins
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.generation_view_mode);
        app.insert_resource(ActiveGeneration::default());

        // If the resources already exists, nothing happens, else, add them with default values.
        app.init_resource::<ProcGenKeyBindings>()
            .init_resource::<GenerationControl>()
            .init_resource::<SelectionCursorMarkerSettings>()
            .init_resource::<CursorKeyboardMovement>()
            .init_resource::<CursorKeyboardMovementSettings>();
        match self.config.cursor_ui_mode {
            CursorUiMode::None => (),
            _ => {
                app.init_resource::<GridCursorsUiSettings>();
            }
        }

        app.add_event::<GenerationResetEvent>()
            .add_event::<NodesGeneratedEvent>();

        #[cfg(feature = "egui-edit")]
        app.init_resource::<EditorConfig>()
            .init_resource::<EditorContext>()
            .add_event::<BrushEvent>();

        #[cfg(feature = "picking")]
        app.init_resource::<CursorTargetAssets>()
            .init_resource::<OverCursorMarkerSettings>()
            .add_event::<NodeOverEvent>()
            .add_event::<NodeOutEvent>()
            .add_event::<NodeSelectedEvent>();

        app
            // PostStartup to wait for setup_cursors_overlays to be applied.
            .add_systems(PostStartup, setup_cursor::<C, SelectCursor>)
            // Keybinds and picking events handlers run in PreUpdate
            .add_systems(
                PreUpdate,
                (
                    deselect_from_keybinds,
                    switch_generation_selection_from_keybinds::<C>,
                    move_selection_from_keybinds::<C>,
                ),
            )
            .add_systems(
                Update,
                (
                    update_generation_control,
                    update_active_generation::<C>,
                    update_cursors_info_on_cursors_changes::<C>,
                ),
            );
        add_named_observer!(update_cursors_info_on_generation_reset::<C>, app);
        add_named_observer!(update_cursors_info_on_generated_nodes::<C>, app);

        #[cfg(feature = "picking")]
        {
            app.add_systems(Startup, setup_picking_assets)
                // PostStartup to wait for setup_cursors_overlays to be applied.
                .add_systems(PostStartup, setup_cursor::<C, OverCursor>)
                .add_systems(
                    Update,
                    (
                        update_cursor_targets_nodes::<C>,
                        (
                            picking_remove_previous_over_cursor::<C>,
                            picking_update_cursors_position::<
                                C,
                                OverCursorMarkerSettings,
                                OverCursor,
                                NodeOverEvent,
                            >,
                            picking_update_cursors_position::<
                                C,
                                SelectionCursorMarkerSettings,
                                SelectCursor,
                                NodeSelectedEvent,
                            >,
                        )
                            .chain(),
                    ),
                );
            add_named_observer!(insert_cursor_picking_handlers_on_grid_nodes::<C>, app);
            add_named_observer!(update_over_cursor_on_generation_reset::<C>, app);
        }

        #[cfg(feature = "egui-edit")]
        app.add_systems(
            Update,
            (
                draw_edition_panel::<C>,
                update_brush,
                update_painting_state,
                paint::<C>,
            )
                .chain()
                .run_if(editor_enabled),
        );

        match self.config.cursor_ui_mode {
            CursorUiMode::None => (),
            CursorUiMode::Panel => {
                app.add_systems(Startup, setup_cursors_panel);
                app.add_systems(PostUpdate, update_selection_cursor_panel_text);
                #[cfg(feature = "picking")]
                app.add_systems(PostUpdate, update_over_cursor_panel_text);
            }
            CursorUiMode::Overlay => {
                app.add_systems(Startup, setup_cursors_overlays);
                app.add_systems(Update, update_cursors_overlays);
            }
        }

        match self.config.generation_view_mode {
            GenerationViewMode::StepByStepTimed {
                steps_count,
                interval_ms,
            } => {
                app.add_systems(
                    Update,
                    (
                        (insert_error_markers_to_new_generations::<C>,),
                        step_by_step_timed_update::<C>,
                        dequeue_generation_updates::<C>,
                    )
                        .chain(),
                );
                app.insert_resource(StepByStepTimed {
                    steps_count,
                    timer: Timer::new(Duration::from_millis(interval_ms), TimerMode::Repeating),
                });
            }
            GenerationViewMode::StepByStepManual => {
                app.add_systems(
                    Update,
                    (
                        (insert_error_markers_to_new_generations::<C>,),
                        step_by_step_input_update::<C>,
                        dequeue_generation_updates::<C>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (generate_all::<C>, dequeue_generation_updates::<C>).chain(),
                );
            }
        }
    }
}

/// Controls how the generation occurs.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationViewMode {
    /// Generates steps by steps and waits at least the specified amount (in milliseconds) between each step.
    StepByStepTimed {
        /// How many steps to run once the timer has finished a cycle
        steps_count: u32,
        /// Time to wait in ms before the next steps
        interval_ms: u64,
    },
    /// Generates step by step and waits for a user input between each step.
    StepByStepManual,
    /// Generates it all at once at the start
    #[default]
    Final,
}

/// Used to track the status of the generation control
#[derive(Eq, PartialEq, Debug)]
pub enum GenerationControlStatus {
    /// Generation control is paused, systems won't automatically step the generation
    Paused,
    /// Generation control is "ongoing", systems can currently step a generator
    Ongoing,
}

/// Read by the systems while generating
#[derive(Resource)]
pub struct GenerationControl {
    /// Current status of the generation
    pub status: GenerationControlStatus,
    /// Indicates whether or not the generator needs to be reinitialized before calling generation operations.
    ///
    /// When using [`GenerationViewMode::Final`], this only controls the first reinitialization per try pool.
    pub need_reinit: bool,
    /// Whether or not the spawning systems do one more generation step when nodes without assets are generated.
    ///
    /// Not used when using [`GenerationViewMode::Final`].
    pub skip_void_nodes: bool,
    /// Whether or not the generation should pause when successful
    pub pause_when_done: bool,
    /// Whether or not the generation should pause when it fails.
    ///
    /// When using [`GenerationViewMode::Final`], this only pauses on the last error of a try pool.
    pub pause_on_error: bool,
    /// Whether or not the generation should pause when it reinitializes
    ///
    /// When using [`GenerationViewMode::Final`], this only pauses on the first reinitialization of a try pool.
    pub pause_on_reinitialize: bool,
}

impl Default for GenerationControl {
    fn default() -> Self {
        Self {
            status: GenerationControlStatus::Paused,
            need_reinit: false,
            skip_void_nodes: true,
            pause_when_done: true,
            pause_on_error: true,
            pause_on_reinitialize: true,
        }
    }
}

/// Resource to track the generation steps when using [`GenerationViewMode::StepByStepTimed`]
#[derive(Resource)]
pub struct StepByStepTimed {
    /// How many steps should be done once the timer has expired
    pub steps_count: u32,
    /// Timer, tracking the time between the steps
    pub timer: Timer,
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
