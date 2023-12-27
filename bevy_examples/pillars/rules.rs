use bevy_ghx_proc_gen::proc_gen::{
    generator::node::{NodeModel, SocketCollection, SocketsCartesian3D},
    grid::direction::Cartesian3D,
};

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian3D>>,
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

    let models_asset_paths: Vec<Option<&str>> = vec![
        None,
        Some("pillar_base"),
        Some("pillar_core"),
        Some("pillar_cap"),
    ];
    let models = vec![
        SocketsCartesian3D::Mono(void).new_model().with_weight(60.),
        SocketsCartesian3D::Simple {
            x_pos: pillar_side,
            x_neg: pillar_side,
            z_pos: pillar_side,
            z_neg: pillar_side,
            y_pos: pillar_base_top,
            y_neg: pillar_base_bottom,
        }
        .new_model(),
        SocketsCartesian3D::Simple {
            x_pos: pillar_side,
            x_neg: pillar_side,
            z_pos: pillar_side,
            z_neg: pillar_side,
            y_pos: pillar_core_top,
            y_neg: pillar_core_bottom,
        }
        .new_model(),
        SocketsCartesian3D::Simple {
            x_pos: pillar_side,
            x_neg: pillar_side,
            z_pos: pillar_side,
            z_neg: pillar_side,
            y_pos: pillar_cap_top,
            y_neg: pillar_cap_bottom,
        }
        .new_model(),
    ];

    sockets
        .add_connections(vec![
            (void, vec![void]),
            (pillar_side, vec![pillar_side, void]),
        ])
        .add_rotated_connection(pillar_base_top, vec![pillar_core_bottom])
        .add_rotated_connection(pillar_core_top, vec![pillar_core_bottom, pillar_cap_bottom])
        .add_rotated_connection(pillar_cap_top, vec![void]);

    (models_asset_paths, models, sockets)
}
