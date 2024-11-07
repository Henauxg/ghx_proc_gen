use bevy_examples::utils::AssetDef;
use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        model::ModelCollection,
        socket::{SocketCollection, SocketsCartesian3D},
    },
    ghx_grid::cartesian::coordinates::Cartesian3D,
};

pub(crate) fn rules_and_assets() -> (
    Vec<Vec<AssetDef>>,
    ModelCollection<Cartesian3D>,
    SocketCollection,
) {
    let mut sockets = SocketCollection::new();

    let void = sockets.create();

    let pillar_side = sockets.create();

    let pillar_base_top = sockets.create();
    let pillar_base_bottom = sockets.create();

    let pillar_core_bottom = sockets.create();
    let pillar_core_top = sockets.create();

    let pillar_cap_bottom = sockets.create();
    let pillar_cap_top = sockets.create();

    let mut models_assets: Vec<Vec<AssetDef>> = Vec::new();
    let mut models = ModelCollection::new();

    models_assets.push(vec![]);
    models
        .create(
            SocketsCartesian3D::Mono(void)
                .to_template()
                .with_weight(60.),
        )
        .with_name("void");

    models_assets.push(vec![AssetDef::new("pillar_base")]);
    models
        .create(SocketsCartesian3D::Simple {
            x_pos: pillar_side,
            x_neg: pillar_side,
            z_pos: pillar_side,
            z_neg: pillar_side,
            y_pos: pillar_base_top,
            y_neg: pillar_base_bottom,
        })
        .with_name("pillar_base");

    models_assets.push(vec![AssetDef::new("pillar_core")]);
    models
        .create(SocketsCartesian3D::Simple {
            x_pos: pillar_side,
            x_neg: pillar_side,
            z_pos: pillar_side,
            z_neg: pillar_side,
            y_pos: pillar_core_top,
            y_neg: pillar_core_bottom,
        })
        .with_name("pillar_core");

    models_assets.push(vec![AssetDef::new("pillar_cap")]);
    models
        .create(SocketsCartesian3D::Simple {
            x_pos: pillar_side,
            x_neg: pillar_side,
            z_pos: pillar_side,
            z_neg: pillar_side,
            y_pos: pillar_cap_top,
            y_neg: pillar_cap_bottom,
        })
        .with_name("pillar_cap");

    sockets
        .add_connections(vec![
            (void, vec![void]),
            (pillar_side, vec![pillar_side, void]),
        ])
        // For this generation, our rotation axis is Y+, so we define connection on the Y axis with `add_rotated_connection` for sockets that still need to be compatible when rotated.
        // Note: But in reality, in this example, we don't really need it. None of our models uses any rotation, apart from ModelRotation::Rot0 (notice that there's no call to `with_rotations` on any of the models).
        // Simply using `add_connections` would give the same result (it allows connections with relative_rotation = Rot0)
        .add_rotated_connections(vec![
            (pillar_base_top, vec![pillar_core_bottom]),
            (pillar_core_top, vec![pillar_core_bottom, pillar_cap_bottom]),
            (pillar_cap_top, vec![void]),
        ]);

    (models_assets, models, sockets)
}
