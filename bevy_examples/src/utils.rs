use std::collections::HashMap;

use bevy::{
    asset::{Asset, AssetServer},
    ecs::{
        query::With,
        system::{Query, Res},
    },
    input::{keyboard::KeyCode, Input},
    render::view::Visibility,
};
use bevy_ghx_proc_gen::grid::view::DebugGridView;

use crate::{fps::FpsRoot, AssetDef, NodeAsset};

pub fn toggle_debug_grid_visibility(
    keys: Res<Input<KeyCode>>,
    mut grid_views: Query<&mut DebugGridView>,
) {
    if keys.just_pressed(KeyCode::F1) {
        for mut view in grid_views.iter_mut() {
            view.display_grid = !view.display_grid;
        }
    }
}

/// Toggle the FPS counter when pressing F2
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

pub fn load_assets<A: Asset>(
    asset_server: &Res<AssetServer>,
    assets_definitions: Vec<Vec<AssetDef>>,
    assets_directory: &str,
    extension: &str,
) -> HashMap<usize, Vec<NodeAsset<A>>> {
    let mut models_assets = HashMap::new();
    for (model_index, assets) in assets_definitions.iter().enumerate() {
        let mut node_assets = Vec::new();
        for asset_def in assets {
            let handle = asset_server.load(format!(
                "{assets_directory}/{}.{extension}",
                asset_def.path()
            ));
            node_assets.push(asset_def.to_asset(handle));
        }
        models_assets.insert(model_index, node_assets);
    }
    models_assets
}
