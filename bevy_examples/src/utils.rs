use bevy::{
    asset::{Asset, AssetServer, Handle},
    ecs::system::Res,
    math::Vec3,
    prelude::EntityCommands,
};

use bevy_ghx_proc_gen::{
    assets::{BundleInserter, ModelAsset, ModelsAssets},
    proc_gen::ghx_grid::cartesian::coordinates::GridDelta,
};

/// Used to define an asset (not yet loaded) for a model: via an asset path, and an optionnal grid offset when spawned in Bevy
#[derive(Clone)]
pub struct ModelAssetDef {
    /// Path of the asset
    pub path: &'static str,
    /// Offset in grid coordinates
    pub grid_offset: GridDelta,
    /// Offset in world coordinates
    pub offset: Vec3,
    pub components_spawner: fn(&mut EntityCommands),
}

impl ModelAssetDef {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            grid_offset: GridDelta::new(0, 0, 0),
            offset: Vec3::ZERO,
            components_spawner: |_| {},
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

    pub fn with_components(mut self, spawn_cmds: fn(&mut EntityCommands)) -> Self {
        self.components_spawner = spawn_cmds;
        self
    }

    pub fn path(&self) -> &'static str {
        self.path
    }
    pub fn offset(&self) -> &GridDelta {
        &self.grid_offset
    }
}

pub fn load_assets<A: Asset>(
    asset_server: &Res<AssetServer>,
    assets_definitions: Vec<Vec<ModelAssetDef>>,
    assets_directory: &str,
    extension: &str,
) -> ModelsAssets<Handle<A>>
where
    Handle<A>: BundleInserter,
{
    let mut models_assets = ModelsAssets::<Handle<A>>::new();
    for (model_index, assets) in assets_definitions.iter().enumerate() {
        for asset_def in assets {
            models_assets.add(
                model_index,
                ModelAsset {
                    assets_bundle: asset_server.load::<A>(format!(
                        "{assets_directory}/{}.{extension}",
                        asset_def.path()
                    )),
                    grid_offset: asset_def.grid_offset.clone(),
                    world_offset: asset_def.offset,
                    spawn_commands: asset_def.components_spawner.clone(),
                },
            )
        }
    }
    models_assets
}
