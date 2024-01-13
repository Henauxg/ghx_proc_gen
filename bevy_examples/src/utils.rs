use bevy::{
    asset::{Asset, AssetServer, Handle},
    ecs::{
        query::With,
        system::{Query, Res},
    },
    input::{keyboard::KeyCode, Input},
    render::view::Visibility,
};
use bevy_ghx_proc_gen::{
    gen::{AssetHandles, ModelAsset, RulesModelsAssets},
    grid::view::DebugGridView,
    proc_gen::grid::direction::GridDelta,
};

use crate::fps::FpsRoot;

/// Toggles the debug grids visibility when pressing F1
pub fn toggle_debug_grids_visibilities(
    keys: Res<Input<KeyCode>>,
    mut grid_views: Query<&mut DebugGridView>,
) {
    if keys.just_pressed(KeyCode::F1) {
        for mut view in grid_views.iter_mut() {
            view.display_grid = !view.display_grid;
        }
    }
}

/// Toggles the FPS counter when pressing F2
pub fn toggle_fps_counter(
    mut fps_ui: Query<&mut Visibility, With<FpsRoot>>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::F2) {
        let mut vis = fps_ui.single_mut();
        *vis = match *vis {
            Visibility::Hidden => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}

/// Used to define an asset (not yet loaded) for a model: via an asset path, and an optionnal grid offset when spawned in Bevy
#[derive(Clone)]
pub struct AssetDef {
    path: &'static str,
    offset: GridDelta,
}

impl AssetDef {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            offset: GridDelta::new(0, 0, 0),
        }
    }

    pub fn with_offset(mut self, offset: GridDelta) -> Self {
        self.offset = offset;
        self
    }

    pub fn to_asset<A: AssetHandles>(&self, asset_ref: A) -> ModelAsset<A> {
        ModelAsset {
            handles: asset_ref,
            offset: self.offset.clone(),
        }
    }

    pub fn path(&self) -> &'static str {
        self.path
    }
    pub fn offset(&self) -> &GridDelta {
        &self.offset
    }
}

/// Simply load assets with the asset_server and return a map that gives assets from a model_index
pub fn load_assets<S: Asset>(
    asset_server: &Res<AssetServer>,
    assets_definitions: Vec<Vec<AssetDef>>,
    assets_directory: &str,
    extension: &str,
) -> RulesModelsAssets<Handle<S>> {
    let mut models_assets = RulesModelsAssets::new();
    for (model_index, assets) in assets_definitions.iter().enumerate() {
        for asset_def in assets {
            models_assets.add(
                model_index,
                ModelAsset {
                    handles: asset_server.load(format!(
                        "{assets_directory}/{}.{extension}",
                        asset_def.path()
                    )),
                    offset: asset_def.offset.clone(),
                },
            )
        }
    }
    models_assets
}
