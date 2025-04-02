use bevy::{
    asset::Handle,
    ecs::system::EntityCommands,
    image::Image,
    math::{Quat, Vec3},
    pbr::{Material, MeshMaterial3d, StandardMaterial},
    prelude::Mesh3d,
    render::mesh::Mesh,
    scene::{Scene, SceneRoot},
    sprite::Sprite,
    transform::components::Transform,
    utils::default,
};
use ghx_proc_gen::generator::model::ModelRotation;

use super::assets::BundleInserter;

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Z+
impl BundleInserter for Handle<Image> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert((
            Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_z(rotation.rad())),
            Sprite {
                image: self.clone(),
                ..default()
            },
        ));
    }
}

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Y+
impl BundleInserter for Handle<Scene> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert((
            Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            SceneRoot(self.clone()),
        ));
    }
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`Material`]
#[derive(Default, Clone, Debug)]
pub struct MaterialMesh<M: Material> {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Material handle
    pub material: Handle<M>,
}

/// Custom type to store [`Handle`] to a [`Mesh`] asset and its [`StandardMaterial`]
///
/// Specialization of [`MaterialMesh`] with [`StandardMaterial`]
#[derive(Default, Clone, Debug)]
pub struct PbrMesh {
    /// Mesh handle
    pub mesh: Handle<Mesh>,
    /// Standard material handle
    pub material: Handle<StandardMaterial>,
}

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Y+
impl<M: Material + Default> BundleInserter for MaterialMesh<M> {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert((
            Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            Mesh3d(self.mesh.clone()),
            MeshMaterial3d(self.material.clone()),
        ));
    }
}

/// **WARNING**: Assumes a specific `Rotation Axis` for the `Models`: Y+
impl BundleInserter for PbrMesh {
    fn insert_bundle(
        &self,
        commands: &mut EntityCommands,
        translation: Vec3,
        scale: Vec3,
        rotation: ModelRotation,
    ) {
        commands.insert((
            Transform::from_translation(translation)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_y(rotation.rad())),
            Mesh3d(self.mesh.clone()),
            MeshMaterial3d(self.material.clone()),
        ));
    }
}
