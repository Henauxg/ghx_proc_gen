use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, PostStartup, PostUpdate, PreUpdate, Startup, Update},
    ecs::{
        schedule::{apply_deferred, IntoSystemConfigs},
        system::Resource,
    },
    input::keyboard::KeyCode,
    render::color::Color,
    time::{Timer, TimerMode},
};
use ghx_proc_gen::grid::direction::CoordinateSystem;

use self::{
    cursor::{
        deselect_from_keybinds, move_selection_from_keybinds, setup_cursor, setup_cursors_overlays,
        setup_cursors_panel, switch_generation_selection_from_keybinds,
        update_cursors_info_from_generation_events, update_cursors_info_on_cursors_changes,
        update_cursors_overlays, update_selection_cursor_panel_text, CursorKeyboardMoveCooldown,
        SelectCursor, SelectionCursorMarkerSettings,
    },
    generation::{
        generate_all, insert_error_markers_to_new_generations,
        insert_void_nodes_to_new_generations, step_by_step_input_update, step_by_step_timed_update,
        update_active_generation, update_generation_control, update_generation_view,
        ActiveGeneration, GenerationEvent,
    },
    picking::{
        picking_remove_previous_over_cursor, setup_picking_assets, update_cursor_targets_nodes,
        update_over_cursor_from_generation_events, update_over_cursor_panel_text,
        CursorTargetAssets, NodeOutEvent, OverCursor, OverCursorMarkerSettings,
    },
};
use super::{
    assets::NoComponents, insert_default_bundle_to_spawned_nodes, spawn_node, AssetSpawner,
    AssetsBundleSpawner, ComponentSpawner,
};

#[cfg(feature = "picking")]
use bevy_mod_picking::PickableBundle;

#[cfg(feature = "picking")]
use self::picking::{
    insert_cursor_picking_handlers_to_grid_nodes, picking_update_cursors_position, NodeOverEvent,
    NodeSelectedEvent,
};

#[cfg(feature = "picking")]
pub mod picking;

#[cfg(feature = "egui-edit")]
use self::egui_editor::{draw_cursor_edit_window, paint, update_painting_state, EditorContext};

#[cfg(feature = "egui-edit")]
pub mod egui_editor;

pub mod cursor;
pub mod generation;

const CURSOR_KEYS_MOVEMENT_COOLDOWN_MS: u64 = 55;

#[derive(Default, Debug, PartialEq, Eq)]
pub enum CursorUiMode {
    None,
    Panel,
    #[default]
    Overlay,
}

#[derive(Resource, Debug)]
pub struct GridCursorsUiSettings {
    pub font_size: f32,
    pub background_color: Color,
    pub text_color: Color,
}

impl Default for GridCursorsUiSettings {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            background_color: Color::BLACK.with_a(0.45),
            text_color: Color::WHITE,
        }
    }
}

/// A [`Plugin`] useful for debug/analysis/demo. It mainly run [`Generator`] components and spawn the generated model's [`crate::gen::assets::ModelAsset`]
///
/// It takes in a [`GenerationViewMode`] to control how the generators components will be run.
///
/// It also uses the following `Resources`: [`ProcGenKeyBindings`] and [`GenerationControl`] (and will init them to their defaults if not inserted by the user).
pub struct ProcGenDebugPlugin<
    C: CoordinateSystem,
    A: AssetsBundleSpawner,
    T: ComponentSpawner = NoComponents,
> {
    generation_view_mode: GenerationViewMode,
    cursor_ui_mode: CursorUiMode,
    typestate: PhantomData<(C, A, T)>,
}

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> ProcGenDebugPlugin<C, A, T> {
    /// Plugin constructor
    pub fn new(generation_view_mode: GenerationViewMode, cursor_ui_mode: CursorUiMode) -> Self {
        Self {
            generation_view_mode,
            cursor_ui_mode,
            typestate: PhantomData,
        }
    }
}

impl<C: CoordinateSystem, A: AssetsBundleSpawner, T: ComponentSpawner> Plugin
    for ProcGenDebugPlugin<C, A, T>
{
    fn build(&self, app: &mut App) {
        app.insert_resource(self.generation_view_mode);
        app.insert_resource(ActiveGeneration::default());

        // If the resources already exists, nothing happens, else, add them with default values.
        app.init_resource::<ProcGenKeyBindings>();
        app.init_resource::<GenerationControl>();
        app.init_resource::<OverCursorMarkerSettings>();
        app.init_resource::<SelectionCursorMarkerSettings>();
        app.init_resource::<CursorKeyboardMoveCooldown>();
        match self.cursor_ui_mode {
            CursorUiMode::None => (),
            _ => {
                app.init_resource::<GridCursorsUiSettings>();
            }
        }
        #[cfg(feature = "egui-edit")]
        app.init_resource::<EditorContext>();
        #[cfg(feature = "picking")]
        app.init_resource::<CursorTargetAssets>();

        app.add_event::<GenerationEvent>();
        #[cfg(feature = "picking")]
        app.add_event::<NodeOverEvent>()
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
            )
            .add_systems(PostUpdate, update_cursors_info_from_generation_events::<C>);

        #[cfg(feature = "picking")]
        app.add_systems(Startup, setup_picking_assets)
            // PostStartup to wait for setup_cursors_overlays to be applied.
            .add_systems(PostStartup, setup_cursor::<C, OverCursor>)
            .add_systems(
                Update,
                (
                    insert_default_bundle_to_spawned_nodes::<PickableBundle>,
                    (
                        update_cursor_targets_nodes::<C>,
                        apply_deferred,
                        insert_cursor_picking_handlers_to_grid_nodes::<C>,
                    )
                        .chain(),
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
            )
            .add_systems(
                PostUpdate,
                update_over_cursor_from_generation_events::<C>
                    .before(update_cursors_info_from_generation_events::<C>),
            );

        #[cfg(feature = "egui-edit")]
        app.add_systems(
            Update,
            (
                draw_cursor_edit_window::<C>,
                (update_painting_state, paint::<C>).chain(),
            ),
        );

        match self.cursor_ui_mode {
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

        match self.generation_view_mode {
            GenerationViewMode::StepByStepTimed {
                steps_count,
                interval_ms,
            } => {
                app.add_systems(
                    Update,
                    (
                        (
                            insert_error_markers_to_new_generations::<C>,
                            insert_void_nodes_to_new_generations::<C, A, T>,
                        ),
                        step_by_step_timed_update::<C>,
                        update_generation_view::<C, A, T>,
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
                        (
                            insert_error_markers_to_new_generations::<C>,
                            insert_void_nodes_to_new_generations::<C, A, T>,
                        ),
                        step_by_step_input_update::<C>,
                        update_generation_view::<C, A, T>,
                    )
                        .chain(),
                );
            }
            GenerationViewMode::Final => {
                app.add_systems(
                    Update,
                    (generate_all::<C>, update_generation_view::<C, A, T>).chain(),
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
#[derive(Resource, Eq, PartialEq, Debug)]
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

/// Resource available to override the default keybindings used by the [`ProcGenDebugPlugin`]
#[derive(Resource)]
pub struct ProcGenKeyBindings {
    pub prev_node: KeyCode,
    pub next_node: KeyCode,
    pub cursor_x_axis: KeyCode,
    pub cursor_y_axis: KeyCode,
    pub cursor_z_axis: KeyCode,
    pub deselect: KeyCode,
    pub switch_grid: KeyCode,

    /// Key to pause/unpause the current [`GenerationControlStatus`]
    pub pause_toggle: KeyCode,
    /// Key used only with [`GenerationViewMode::StepByStepPaused`] to step once per press
    pub step: KeyCode,
    /// Key used only with [`GenerationViewMode::StepByStepPaused`] to step continuously as long as pressed
    pub continuous_step: KeyCode,
}

impl Default for ProcGenKeyBindings {
    fn default() -> Self {
        Self {
            prev_node: KeyCode::Left,
            next_node: KeyCode::Right,
            cursor_x_axis: KeyCode::X,
            cursor_y_axis: KeyCode::Y,
            cursor_z_axis: KeyCode::Z,
            deselect: KeyCode::Escape,
            switch_grid: KeyCode::Tab,
            pause_toggle: KeyCode::Space,
            step: KeyCode::Down,
            continuous_step: KeyCode::Up,
        }
    }
}
