use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        node::{NodeModel, SocketId, SocketsCartesian3D},
        rules::SocketConnections,
    },
    grid::direction::Cartesian3D,
};

const VOID: SocketId = 0;

const PILLAR_SIDE: SocketId = 1;

const PILLAR_BASE_TOP: SocketId = 2;
const PILLAR_BASE_BOTTOM: SocketId = 3;

const PILLAR_CORE_BOTTOM: SocketId = 4;
const PILLAR_CORE_TOP: SocketId = 5;

const PILLAR_CAP_BOTTOM: SocketId = 6;
const PILLAR_CAP_TOP: SocketId = 7;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian3D>>,
    Vec<SocketConnections>,
) {
    let models_asset_paths: Vec<Option<&str>> = vec![
        None,
        Some("pillar_base"),
        Some("pillar_core"),
        Some("pillar_cap"),
    ];
    let models = vec![
        SocketsCartesian3D::Mono(VOID).new_model().with_weight(30.),
        SocketsCartesian3D::Simple {
            x_pos: PILLAR_SIDE,
            x_neg: PILLAR_SIDE,
            z_pos: PILLAR_SIDE,
            z_neg: PILLAR_SIDE,
            y_pos: PILLAR_BASE_TOP,
            y_neg: PILLAR_BASE_BOTTOM,
        }
        .new_model(),
        SocketsCartesian3D::Simple {
            x_pos: PILLAR_SIDE,
            x_neg: PILLAR_SIDE,
            z_pos: PILLAR_SIDE,
            z_neg: PILLAR_SIDE,
            y_pos: PILLAR_CORE_TOP,
            y_neg: PILLAR_CORE_BOTTOM,
        }
        .new_model(),
        SocketsCartesian3D::Simple {
            x_pos: PILLAR_SIDE,
            x_neg: PILLAR_SIDE,
            z_pos: PILLAR_SIDE,
            z_neg: PILLAR_SIDE,
            y_pos: PILLAR_CAP_TOP,
            y_neg: PILLAR_CAP_BOTTOM,
        }
        .new_model(),
    ];
    let sockets_connections = vec![
        (VOID, vec![VOID]),
        (PILLAR_SIDE, vec![PILLAR_SIDE, VOID]),
        (PILLAR_BASE_TOP, vec![PILLAR_CORE_BOTTOM]),
        (PILLAR_CORE_TOP, vec![PILLAR_CORE_BOTTOM, PILLAR_CAP_BOTTOM]),
        (PILLAR_CAP_TOP, vec![VOID]),
    ];
    (models_asset_paths, models, sockets_connections)
}
