use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use bevy::{ecs::system::EntityCommands, math::Vec3};
use ghx_proc_gen::{
    generator::model::{ModelIndex, ModelRotation},
    ghx_grid::cartesian::coordinates::GridDelta,
};

/// Defines a struct which can spawn components on an Entity (for example, a [`bevy::sprite::Sprite`], a [`bevy::scene::SceneRoot`], ...).
pub trait BundleInserter: Sync + Send + Default + 'static {
    /// From the `BundleSpawner` own's struct data and a position, scale and rotation, can modify the spawned node `Entity`
    fn insert_bundle(
        &self,
        command: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    );
}

/// Represents spawnable asset(s) & component(s) for a model.
///
/// They will be spawned every time this model is generated. One `ModelAsset` will spawn exactly one [`bevy::prelude::Entity`] (but note that one Model may have more than one `ModelAsset`).
#[derive(Clone, Debug)]
// pub struct ModelAsset<A: BundleSpawner, T: BundleSpawner = NoComponents> {
pub struct ModelAsset<A: BundleInserter> {
    /// Stores handle(s) to the asset(s) and spawns their bundle
    pub assets_bundle: A,
    /// Spawn commands to add additional components to a spawned model
    pub spawn_commands: fn(&mut EntityCommands),
    /// Grid offset from the generated grid node position. Added to `offset`.
    pub grid_offset: GridDelta,
    /// World offset from the generated grid node position. Added to `grid_offset`.
    pub world_offset: Vec3,
}

/// Defines a map which links a `Model` via its [`ModelIndex`] to his spawnable(s) [`ModelAsset`]
#[derive(Debug)]
pub struct ModelsAssets<A: BundleInserter> {
    /// Only contains a ModelIndex if there are some assets for it. One model may have multiple [`ModelAsset`].
    map: HashMap<ModelIndex, Vec<ModelAsset<A>>>,
}
impl<A: BundleInserter> Deref for ModelsAssets<A> {
    type Target = HashMap<ModelIndex, Vec<ModelAsset<A>>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl<A: BundleInserter> DerefMut for ModelsAssets<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<A: BundleInserter> ModelsAssets<A> {
    /// Create a new ModelsAssets with an empty map
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Create a new ModelsAssets with an existing map
    pub fn new_from_map(map: HashMap<ModelIndex, Vec<ModelAsset<A>>>) -> Self {
        Self { map }
    }

    /// Adds a [`ModelAsset`] with no grid offset, to the model `index`
    pub fn add_asset(&mut self, index: ModelIndex, asset: A) {
        let model_asset = ModelAsset {
            assets_bundle: asset,
            grid_offset: Default::default(),
            world_offset: Vec3::ZERO,
            spawn_commands: |_| {},
        };
        self.add(index, model_asset);
    }

    /// Adds a [`ModelAsset`] to the model `index`
    pub fn add(&mut self, index: ModelIndex, model_asset: ModelAsset<A>) {
        match self.get_mut(&index) {
            Some(assets) => {
                assets.push(model_asset);
            }
            None => {
                self.insert(index, vec![model_asset]);
            }
        }
    }
}
