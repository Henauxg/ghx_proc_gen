use std::{marker::PhantomData, time::Duration};

use bevy::{
    app::{App, Plugin, PostUpdate, PreUpdate, Startup, Update},
    ecs::{schedule::IntoSystemConfigs, system::Resource},
    input::keyboard::KeyCode,
    render::color::Color,
    time::{Timer, TimerMode},
};
use ghx_proc_gen::grid::direction::CoordinateSystem;

use self::{
    cursor::{
        insert_selection_cursor_to_new_generations, keybinds_update_selection_cursor_position,
        setup_cursors_overlays, setup_cursors_panel, update_cursor_info_on_cursor_changes,
        update_cursors_overlay, update_selection_cursor_panel_text, CursorMoveCooldown,
        SelectionCursor, SelectionCursorInfo, SelectionCursorOverlayText,
    },
    generation::{
        generate_all, insert_void_nodes_to_new_generations, step_by_step_input_update,
        step_by_step_timed_update, update_generation_control, update_generation_view,
    },
    picking::{update_over_cursor_panel_text, OverCursorOverlayText},
};
use super::{
    assets::NoComponents, insert_default_bundle_to_spawned_nodes, spawn_node, AssetSpawner,
    AssetsBundleSpawner, ComponentSpawner, SpawnedNode,
};

#[cfg(feature = "picking")]
use bevy_mod_picking::PickableBundle;

#[cfg(feature = "picking")]
use self::picking::{
    insert_grid_cursor_picking_handlers_to_spawned_nodes, insert_over_cursor_to_new_generations,
    picking_update_cursors_position, NodeOverEvent, NodeSelectedEvent, OverCursor, OverCursorInfo,
};

#[cfg(feature = "picking")]
pub mod picking;

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
pub struct GridCursorsUiConfiguration {
    pub font_size: f32,
    pub background_color: Color,
    pub text_color: Color,
}

impl Default for GridCursorsUiConfiguration {
    fn default() -> Self {
        Self {
            font_size: 15.0,
            background_color: Color::BLACK.with_a(0.4),
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

        // If the resources already exists, nothing happens, else, add them with default values.
        app.init_resource::<ProcGenKeyBindings>();
        app.init_resource::<GenerationControl>();
        match self.cursor_ui_mode {
            CursorUiMode::None => (),
            _ => {
                app.init_resource::<GridCursorsUiConfiguration>();
            }
        }

        app.insert_resource(CursorMoveCooldown(Timer::new(
            Duration::from_millis(CURSOR_KEYS_MOVEMENT_COOLDOWN_MS),
            TimerMode::Once,
        )));

        #[cfg(feature = "picking")]
        app.add_event::<NodeOverEvent>()
            .add_event::<NodeSelectedEvent>();

        app.add_systems(
            Update,
            (
                update_generation_control,
                insert_selection_cursor_to_new_generations::<C>,
            ),
        );
        #[cfg(feature = "picking")]
        app.add_systems(
            Update,
            (
                insert_over_cursor_to_new_generations::<C>,
                insert_grid_cursor_picking_handlers_to_spawned_nodes::<C>,
                insert_default_bundle_to_spawned_nodes::<PickableBundle>,
            ),
        );

        // Keybinds and picking events handlers run in PreUpdate
        app.add_systems(PreUpdate, keybinds_update_selection_cursor_position::<C>);
        #[cfg(feature = "picking")]
        app.add_systems(
            Update,
            (
                picking_update_cursors_position::<C, OverCursor, NodeOverEvent>,
                picking_update_cursors_position::<C, SelectionCursor, NodeSelectedEvent>,
                update_cursor_info_on_cursor_changes::<C, OverCursor, OverCursorInfo>,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            update_cursor_info_on_cursor_changes::<C, SelectionCursor, SelectionCursorInfo>,
        );
        match self.cursor_ui_mode {
            CursorUiMode::None => (),
            CursorUiMode::Panel => {
                app.add_systems(Startup, setup_cursors_panel)
                    .add_systems(PostUpdate, update_selection_cursor_panel_text);
                #[cfg(feature = "picking")]
                app.add_systems(PostUpdate, update_over_cursor_panel_text);
            }
            CursorUiMode::Overlay => {
                app.add_systems(Startup, setup_cursors_overlays);
                app.add_systems(
                    Update,
                    update_cursors_overlay::<
                        SelectionCursor,
                        SelectionCursorInfo,
                        SelectionCursorOverlayText,
                    >,
                );
                #[cfg(feature = "picking")]
                app.add_systems(
                    Update,
                    update_cursors_overlay::<OverCursor, OverCursorInfo, OverCursorOverlayText>,
                );
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
                        insert_void_nodes_to_new_generations::<C, A, T>,
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
            GenerationViewMode::StepByStepPaused => {
                app.add_systems(
                    Update,
                    (
                        insert_void_nodes_to_new_generations::<C, A, T>,
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
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum GenerationViewMode {
    /// Generates steps by steps and waits at least the specified amount (in milliseconds) between each step.
    StepByStepTimed {
        /// How many steps to run once the timer has finished a cycle
        steps_count: u32,
        /// Time to wait in ms before the next steps
        interval_ms: u64,
    },
    /// Generates step by step and waits for a user input between each step.
    StepByStepPaused,
    /// Generates it all at once at the start
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
    /// Whether or not the spawning systems do one more generation step when nodes without assets are generated.
    pub skip_void_nodes: bool,
    /// Whether or not the generation should pause when successful
    pub pause_when_done: bool,
    /// Whether or not the generation should pause when it fails
    pub pause_on_error: bool,
}

impl Default for GenerationControl {
    fn default() -> Self {
        Self {
            status: GenerationControlStatus::Ongoing,
            skip_void_nodes: true,
            pause_when_done: true,
            pause_on_error: true,
        }
    }
}

impl GenerationControl {
    /// Create a new `GenerationControl` with the status set to [`GenerationControlStatus::Ongoing`]
    pub fn new(skip_void_nodes: bool, pause_when_done: bool, pause_on_error: bool) -> Self {
        Self {
            status: GenerationControlStatus::Ongoing,
            skip_void_nodes,
            pause_on_error,
            pause_when_done,
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

    /// Key to unpause the current [`GenerationControlStatus`]
    pub unpause: KeyCode,
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
            unpause: KeyCode::Space,
            step: KeyCode::Down,
            continuous_step: KeyCode::Up,
        }
    }
}
