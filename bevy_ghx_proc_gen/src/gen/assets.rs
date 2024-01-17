use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use bevy::{
    ecs::{component::Component, system::EntityCommands},
    math::Vec3,
};
use ghx_proc_gen::{
    generator::model::{ModelIndex, ModelRotation},
    grid::direction::GridDelta,
};

/// Defines a struct which can spawn an assets [`bevy::prelude::Bundle`] (for example, a [`bevy::prelude::SpriteBundle`], a [`bevy::prelude::PbrBundle`], a [`bevy::prelude::SceneBundle`], ...).
pub trait AssetsBundleSpawner: Sync + Send + 'static {
    /// From the `AssetsBundleSpawner` own data, a position, a scale and a rotation, inserts a [`bevy::prelude::Bundle`] into the spawned node `Entity`
    fn insert_bundle(
        &self,
        command: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    );
}

/// Trait used to represent a generic [`Component`]/[`bevy::prelude::Bundle`] container.
///
/// Can be used to store custom components in [`ModelAsset`].
pub trait ComponentSpawner: Sync + Send + 'static {
    /// Insert [`Component`] and/or [`bevy::prelude::Bundle`] into an [`bevy::prelude::Entity`]
    fn insert(&self, commands: &mut EntityCommands);
}

/// Default implementation of [`ComponentSpawner`] which does nothing.
///
/// `Insert` will not even be called if your [`ModelAsset`] don't have components.
#[derive(Clone)]
pub struct NoComponents;
impl ComponentSpawner for NoComponents {
    fn insert(&self, _commands: &mut EntityCommands) {}
}

/// Represents spawnable asset(s) & component(s) for a model.
///
/// They will be spawned every time this model is generated. One `ModelAsset` will spawn exactly one [`bevy::prelude::Entity`].
#[derive(Clone)]
pub struct ModelAsset<A: AssetsBundleSpawner, T: ComponentSpawner = NoComponents> {
    /// Stores handle(s) to the asset(s) and spawns their bundle
    pub assets_bundle: A,
    /// Optionnal vector of [`ComponentSpawner`] that will be spawned for this model
    pub components: Vec<T>,
    /// Grid offset from the generated grid node position. Added to `offset`.
    pub grid_offset: GridDelta,
    /// World offset from the generated grid node position. Added to `grid_offset`.
    pub offset: Vec3,
}

/// Defines a map which links a `Model` via its [`ModelIndex`] to his spawnable [`ModelAsset`]
pub struct RulesModelsAssets<A: AssetsBundleSpawner, T: ComponentSpawner = NoComponents> {
    /// Only contains a ModelIndex if there are some assets for it. One model may have multiple [`ModelAsset`].
    map: HashMap<ModelIndex, Vec<ModelAsset<A, T>>>,
}
impl<A: AssetsBundleSpawner, T: ComponentSpawner> Deref for RulesModelsAssets<A, T> {
    type Target = HashMap<ModelIndex, Vec<ModelAsset<A, T>>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl<A: AssetsBundleSpawner, T: ComponentSpawner> DerefMut for RulesModelsAssets<A, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<A: AssetsBundleSpawner, T: ComponentSpawner> RulesModelsAssets<A, T> {
    /// Create a new RulesModelsAssets with an empty map
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    /// Create a new RulesModelsAssets with an existing map
    pub fn new_from_map(map: HashMap<ModelIndex, Vec<ModelAsset<A, T>>>) -> Self {
        Self { map }
    }

    /// Adds a [`ModelAsset`] with no grid offset, to the model `index`
    pub fn add_asset(&mut self, index: ModelIndex, asset: A) {
        let model_asset = ModelAsset {
            assets_bundle: asset,
            grid_offset: Default::default(),
            offset: Vec3::ZERO,
            components: Vec::new(),
        };
        self.add(index, model_asset);
    }

    /// Adds a [`ModelAsset`] to the model `index`
    pub fn add(&mut self, index: ModelIndex, model_asset: ModelAsset<A, T>) {
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

/// Stores information needed to spawn assets from a [`ghx_proc_gen::generator::Generator`]
#[derive(Component)]
pub struct AssetSpawner<A: AssetsBundleSpawner, T: ComponentSpawner = NoComponents> {
    /// Link a `Model` via its [`ModelIndex`] to his spawnable assets (can be shared by multiple [`AssetSpawner`])
    pub assets: Arc<RulesModelsAssets<A, T>>,
    /// Size of a node in world units
    pub node_size: Vec3,
    /// Scale of the assets when spawned
    pub spawn_scale: Vec3,
    /// Whether to offset the z coordinate of spawned nodes from the y coordinate (used for 2d ordering of sprites)
    pub z_offset_from_y: bool,
}

impl<A: AssetsBundleSpawner, T: ComponentSpawner> AssetSpawner<A, T> {
    /// Constructor for a `AssetSpawner`, `z_offset_from_y` defaults to `false`
    pub fn new(
        models_assets: RulesModelsAssets<A, T>,
        node_size: Vec3,
        spawn_scale: Vec3,
    ) -> AssetSpawner<A, T> {
        Self {
            node_size,
            assets: Arc::new(models_assets),
            spawn_scale,
            z_offset_from_y: false,
        }
    }

    /// Sets the `z_offset_from_y` value
    pub fn with_z_offset_from_y(mut self, z_offset_from_y: bool) -> Self {
        self.z_offset_from_y = z_offset_from_y;
        self
    }
}
