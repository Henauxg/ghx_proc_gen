# Changelog

## Version 0.4.0 (2024-11-07)

###  All crates

- Updated ghx_grid to 0.4
- Updated bevy to 0.14
  - Thanks to @a-panda-miner for the PR [#4](https://github.com/Henauxg/ghx_proc_gen/pull/4)

### `bevy_ghx_proc_gen` crate:

- Fixed: entities with `CursorTarget` are now filtered out from spawned nodes utils systems (`insert_bundle_from_resource_to_spawned_nodes`, `insert_default_bundle_to_spawned_nodes`)

## Version 0.3.0 (2024-11-07)

###  All crates

- Updated ghx_grid from 0.2.0 to 0.3.2: the generator is now generic over the grid trait
  - Thanks to @c6p for the PR https://github.com/Henauxg/ghx_proc_gen/pull/2
  - See the ghx_grid changelog here https://github.com/Henauxg/ghx_grid/blob/main/CHANGELOG.md

### `ghx_proc_gen` crate:

- Expose `ghx_grid` as public
- Enable `ghx_grid` `bevy` and `reflect` features when these features are enabled in `ghx_proc_gen`

### `bevy_ghx_proc_gen` crate:

- The bevy plugins are only implemented for `CartesianCoordinates`
- Enable `bevy/bevy_ui` when enabling the `debug-plugin` feature
- Enable `bevy_ghx_grid/reflect` when enabling the `reflect` feature

### Examples

- Update to use bevy_ghx_grid bundled within bevy_ghx_proc_gen

## Version 0.2.0 (2024-05-17)

### All crates

- Updated to bevy 0.13

### `ghx_proc_gen` crate:

- When `bevy` feature is enabled, `QueuedObserver` and `QueuedStatefulObserverare` now derive `Component`
- Model names are now part of a new `models-names` feature separate from `debug-traces`
- Added `ModelCollection` to create models from
- `Rules`:
    - Added `ModelVariantRef`
    - Added `weight` and `name_str` getters
    - Added new type `ModelInfo`
    - `RulesBuilder` now takes in a `ModelCollection`
- `GeneratorBuilder`:
    - `build` is now faillible (it checks the initial nodes + the rules for obvious impossible generations) Initialization now runs during the generator creation and
    - Added `build_collected`
    - Added `with_initial_nodes` and `with_initial_grid`
    - Can now create observers from a `GeneratorBuilder`
    - Added `NodeSetError` and `InvalidGridSize`
- `Generator`
    - Renamed `get_seed` to  `seed`
    - Added `set_and_propagate` and `set_and_propagate_collected`
    - Renamed `generate_collected` to `generate_grid`
    - Renamed `GridNode` to `GeneratedNode`
    - Functions `generate`, `generate_collected` and `generate_grid` now return how many tries it took to successfully generate
    - Added getter/setter for `max_retry_count`
    - Added `get_models_variations_on` and `get_models_on`
    - Added `ModelVariations`
    - Added `pub` `to_grid_data`
- Errors:
    - Added `GeneratorBuilderError`
    - Renamed `RulesError` to `RulesBuilderError`
    - Renamed `GenerationError` to `GeneratorError`
    - Added derive of Clone & Copy for `GeneratorError` and `RulesBuilderError`
- Updated & improved documentation
- Improved memory usage (orginal models sockets were not discarded during generation, the `Rules` now only keep what is strictly necessary)
- Examples:
    - Renamed `checkerboard` examples to `chessboard` and use the new APIs to set bottom-left tile to be a black tile

Grid:

- Extracted grid types `GridDefinition`, `GridData`, `Direction`, ... to their own crate `ghx_grid` and extracted the `GridDebugPlugin` to its own crate: `bevy_ghx_grid`. Parts of the changes are still listed here:
    - `GridDefinition`:
        - Renamed `NodeIndex` to `GridIndex`
        - Added `NodeRef` enum
        - Added `index_from_ref` function
        - Added `get_index_in_direction`
        - Added `coord_system` getter
        - Renamed `get_index_from_pos` to `index_from_pos`
        - Renamed `get_position` to `pos_from_index`
        - Renamed `get_index` to `index_from_coords`
        - Renamed `get_next_index` to `get_next_index_in_direction`
        - Function `directions` is now `pub`
    - `GridData`:
        - Added `set_all_...` functions
        - Function `reset` now accepts Clone types
    - `GridDelta`:
        - Implement Mul<i32>

### `bevy_ghx_proc_gen` crate:

- `Observed` Component removed
- `SpawnedNode`:
    - Renamed to `GridNode`
    - Added `NodeIndex` field
- `GridDebugPlugin`:
    - Renamed `DebugGridView2d`/`DebugGridView3d`  to  `DebugGridView2dBundle`/`DebugGridView3dBundle`
    - Renamed `DebugGridViewConfig3d`/`DebugGridViewConfig2d` to `DebugGridView3d`/`DebugGridView2d`
    - Renamed `Marker` to `GridMarker`
    - Changed `MarkerEvent` to `MarkerDespawnEvent`
    - `GridMarkers` are now created via ECS commands and now have a `Transform` automatically added to them.
    - Debug grids views are now properly scaled/rotated with the grid transform
    - Debug grids for Camera3d are now drawn using Gizmos too
- `ProcGenDebugPlugin`, major ergonomic update: cursors & multi-generations
    - Added cursors, with customizable UI via `CursorUiMode` (Custom, Static-UI panel, or World-UI overlay)
        - Cursors ui style can be configured through `GridCursorsUiSettings`, `SelectionCursorMarkerSettings` & `OverCursorMarkerSettings` Resources
        - Selection cursor keyboard movement can be tweaked by a `CursorKeyboardMoveCooldown` Resource
    - Added a `picking` feature:
        - enables a mouse-over cursor
        - enables picking for the selection cursor
    - Observing multiple generations is now possible properly. Switching is done via picking or keyboard event.
    - `GenerationControl` can now be paused/unpaused
    - Renamed `GenerationViewMode::StepByStepPaused` to `GenerationViewMode::StepByStepManual`
    - Added multiple new public Systems, Events, Components & Resources (`GenerationEvent`, cursor systems, …)
    - Added cursor targets
- Added `toggle_debug_grids_visibilities` utility system
- Added `toggle_grid_markers_visibilities` utility system
- Updated & improved documentation
- Examples:
    - Renamed `checkerboard` examples to `chessboard` and use the new APIs  to set bottom-left tile to be a black tile
    - Examples now use the `picking` feature
    - Added hotkey to toggle camera auto rotation
    - Added hotkey to toggle grid markers visbilities
    - Added hotkey to toggle ui

## Version 0.1.0 (2024-01-19)

Initial release