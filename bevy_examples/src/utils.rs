use bevy::{
    asset::{Asset, AssetServer, Handle},
    ecs::{
        query::With,
        system::{Query, Res},
    },
    input::{keyboard::KeyCode, Input},
    math::Vec3,
    render::view::Visibility,
};
use bevy_ghx_proc_gen::{
    gen::assets::{
        AssetsBundleSpawner, ComponentSpawner, ModelAsset, NoComponents, RulesModelsAssets,
    },
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
pub struct AssetDef<T = NoComponents> {
    path: &'static str,
    grid_offset: GridDelta,
    offset: Vec3,
    components: Vec<T>,
}

impl<T> AssetDef<T> {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            grid_offset: GridDelta::new(0, 0, 0),
            offset: Vec3::ZERO,
            components: Vec::new(),
        }
    }

    pub fn with_grid_offset(mut self, offset: GridDelta) -> Self {
        self.grid_offset = offset;
        self
    }

    pub fn with_offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_component(mut self, component: T) -> Self {
        self.components.push(component);
        self
    }

    pub fn path(&self) -> &'static str {
        self.path
    }
    pub fn offset(&self) -> &GridDelta {
        &self.grid_offset
    }
}

pub fn load_assets<A: Asset, T: ComponentSpawner>(
    asset_server: &Res<AssetServer>,
    assets_definitions: Vec<Vec<AssetDef<T>>>,
    assets_directory: &str,
    extension: &str,
) -> RulesModelsAssets<Handle<A>, T>
where
    Handle<A>: AssetsBundleSpawner,
    T: Clone,
{
    let mut models_assets = RulesModelsAssets::new();
    for (model_index, assets) in assets_definitions.iter().enumerate() {
        for asset_def in assets {
            models_assets.add(
                model_index,
                ModelAsset {
                    assets_bundle: asset_server.load(format!(
                        "{assets_directory}/{}.{extension}",
                        asset_def.path()
                    )),
                    grid_offset: asset_def.grid_offset.clone(),
                    offset: asset_def.offset,
                    components: asset_def.components.clone(),
                },
            )
        }
    }
    models_assets
}
