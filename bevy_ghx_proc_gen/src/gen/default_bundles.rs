use bevy::{
    asset::Handle,
    ecs::system::EntityCommands,
    math::{Quat, Vec3},
    pbr::{Material, MaterialMeshBundle, PbrBundle, StandardMaterial},
    render::{mesh::Mesh, texture::Image},
    scene::{Scene, SceneBundle},
    sprite::SpriteBundle,
    transform::components::Transform,
    utils::default,
};
use ghx_proc_gen::generator::model::ModelRotation;

use super::assets::AssetsBundleSpawner;

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Z+
impl AssetsBundleSpawner for Handle<Image> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(SpriteBundle {
            texture: self.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_z(rotation.rad())),
            ..default()
        });
    }
}

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Y+
impl AssetsBundleSpawner for Handle<Scene> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(SceneBundle {
            scene: self.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            ..default()
        });
    }
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`Material`]
#[derive(Clone)]
pub struct MaterialMesh<M: Material> {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Material handle
    pub material: Handle<M>,
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`StandardMaterial`]
///
/// Specialization of [`MaterialMesh`] with [`StandardMaterial`]
#[derive(Clone)]
pub struct PbrMesh {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Standard material handle
    pub material: Handle<StandardMaterial>,
}

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Y+
impl<M: Material> AssetsBundleSpawner for MaterialMesh<M> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(MaterialMeshBundle {
            mesh: self.mesh.clone(),
            material: self.material.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            ..default()
        });
    }
}

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Y+
impl AssetsBundleSpawner for PbrMesh {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert(PbrBundle {
            mesh: self.mesh.clone(),
            material: self.material.clone(),
            transform: Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            ..default()
        });
    }
}
