use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        node::{NodeModel, SocketsCartesian3D},
        rules::SocketConnections,
    },
    grid::direction::Cartesian3D,
};

const VOID_SIDE: u32 = 0;
const VOID_TOP: u32 = 1;
const VOID_BOTTOM: u32 = 2;

const GROUND_BACK: u32 = 60;
const GROUND_LEFT: u32 = 61;
const GROUND_RIGHT: u32 = 62;
const GROUND_FRONT: u32 = 63;
const GROUND_TOP: u32 = 64;
const GROUND_BOTTOM: u32 = 65;
const GROUND_SIDE: u32 = 66;

const STAIRS_BACK: u32 = 10;
const STAIRS_LEFT: u32 = 11;
const STAIRS_RIGHT: u32 = 12;
const STAIRS_FRONT: u32 = 13;
const STAIRS_TOP: u32 = 14;
const STAIRS_BOTTOM: u32 = 15;
const _STAIRS_SIDE: u32 = 16;

const SIDE_STAIRS_BACK: u32 = 20;
const SIDE_STAIRS_LEFT: u32 = 21;
const SIDE_STAIRS_RIGHT: u32 = 22;
const SIDE_STAIRS_FRONT: u32 = 23;
const SIDE_STAIRS_TOP: u32 = 24;
const SIDE_STAIRS_BOTTOM: u32 = 25;
const _SIDE_STAIRS_SIDE: u32 = 26;

const STAIRS_SUPPORT_BACK: u32 = 30;
const STAIRS_SUPPORT_LEFT: u32 = 31;
const STAIRS_SUPPORT_RIGHT: u32 = 32;
const STAIRS_SUPPORT_FRONT: u32 = 33;
const STAIRS_SUPPORT_TOP: u32 = 34;
const STAIRS_SUPPORT_BOTTOM: u32 = 35;
const STAIRS_SUPPORT_SIDE: u32 = 36;

const PYRAMID_TOP_BACK: u32 = 40;
const PYRAMID_TOP_LEFT: u32 = 41;
const PYRAMID_TOP_RIGHT: u32 = 42;
const PYRAMID_TOP_FRONT: u32 = 43;
const PYRAMID_TOP_TOP: u32 = 44;
const PYRAMID_TOP_BOTTOM: u32 = 45;
const PYRAMID_TOP_SIDE: u32 = 46;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian3D>>,
    Vec<SocketConnections>,
) {
    let models_asset_paths: Vec<Option<&str>> = vec![
        None,
        Some("block"),
        Some("stairs"),
        Some("side_stairs"),
        Some("block"),
        Some("block_wood"),
        // Some("corridor"),
        // Some("corridor_corner"),
        // Some("block_1_third"),
        // Some("full_block"),
        // Some("jagged_stairs"),
        // Some("big_stairs"),
    ];
    let models = vec![
        // Void
        SocketsCartesian3D::Multiple {
            x_pos: vec![VOID_SIDE],
            x_neg: vec![VOID_SIDE],
            z_pos: vec![VOID_SIDE],
            z_neg: vec![VOID_SIDE],
            y_pos: vec![VOID_TOP],
            y_neg: vec![VOID_BOTTOM],
        }
        .new_model()
        .with_weight(0.1),
        // Ground
        SocketsCartesian3D::Multiple {
            x_pos: vec![GROUND_RIGHT, GROUND_SIDE],
            x_neg: vec![GROUND_LEFT, GROUND_SIDE],
            z_pos: vec![GROUND_FRONT, GROUND_SIDE],
            z_neg: vec![GROUND_BACK, GROUND_SIDE],
            y_pos: vec![GROUND_TOP],
            y_neg: vec![GROUND_BOTTOM],
        }
        .new_model()
        .with_all_rotations()
        .with_weight(1.35),
        // Stairs
        SocketsCartesian3D::Multiple {
            x_pos: vec![STAIRS_RIGHT],
            x_neg: vec![STAIRS_LEFT],
            z_pos: vec![STAIRS_FRONT],
            z_neg: vec![STAIRS_BACK],
            y_pos: vec![STAIRS_TOP],
            y_neg: vec![STAIRS_BOTTOM],
        }
        .new_model()
        .with_all_rotations()
        .with_weight(0.3),
        // Side Stairs
        SocketsCartesian3D::Multiple {
            x_pos: vec![SIDE_STAIRS_RIGHT],
            x_neg: vec![SIDE_STAIRS_LEFT],
            z_pos: vec![SIDE_STAIRS_FRONT],
            z_neg: vec![SIDE_STAIRS_BACK],
            y_pos: vec![SIDE_STAIRS_TOP],
            y_neg: vec![SIDE_STAIRS_BOTTOM],
        }
        .new_model()
        .with_all_rotations()
        .with_weight(0.0),
        // Stairs support
        SocketsCartesian3D::Multiple {
            x_pos: vec![STAIRS_SUPPORT_RIGHT, STAIRS_SUPPORT_SIDE],
            x_neg: vec![STAIRS_SUPPORT_LEFT, STAIRS_SUPPORT_SIDE],
            z_pos: vec![STAIRS_SUPPORT_FRONT, STAIRS_SUPPORT_SIDE],
            z_neg: vec![STAIRS_SUPPORT_BACK, STAIRS_SUPPORT_SIDE],
            y_pos: vec![STAIRS_SUPPORT_TOP],
            y_neg: vec![STAIRS_SUPPORT_BOTTOM],
        }
        .new_model()
        .with_weight(0.2),
        // Pyramid top
        SocketsCartesian3D::Multiple {
            x_pos: vec![PYRAMID_TOP_RIGHT, PYRAMID_TOP_SIDE],
            x_neg: vec![PYRAMID_TOP_LEFT, PYRAMID_TOP_SIDE],
            z_pos: vec![PYRAMID_TOP_FRONT, PYRAMID_TOP_SIDE],
            z_neg: vec![PYRAMID_TOP_BACK, PYRAMID_TOP_SIDE],
            y_pos: vec![PYRAMID_TOP_TOP, GROUND_TOP],
            y_neg: vec![PYRAMID_TOP_BOTTOM],
        }
        .new_model()
        .with_weight(0.001),
    ];
    let sockets_connections = vec![
        (GROUND_BACK, vec![GROUND_SIDE]),
        (GROUND_LEFT, vec![GROUND_SIDE]),
        (GROUND_RIGHT, vec![VOID_SIDE, GROUND_SIDE]),
        (GROUND_FRONT, vec![VOID_SIDE, GROUND_SIDE]),
        (GROUND_TOP, vec![VOID_BOTTOM]),
        (VOID_SIDE, vec![VOID_SIDE]),
        (VOID_TOP, vec![VOID_BOTTOM]),
        (STAIRS_BACK, vec![STAIRS_SUPPORT_SIDE]),
        (STAIRS_LEFT, vec![SIDE_STAIRS_RIGHT]),
        (STAIRS_RIGHT, vec![SIDE_STAIRS_LEFT]),
        (STAIRS_FRONT, vec![VOID_SIDE]),
        (STAIRS_TOP, vec![VOID_BOTTOM]),
        (STAIRS_BOTTOM, vec![GROUND_TOP]),
        (SIDE_STAIRS_BACK, vec![STAIRS_SUPPORT_SIDE]),
        (SIDE_STAIRS_LEFT, vec![VOID_SIDE]),
        (SIDE_STAIRS_RIGHT, vec![VOID_SIDE]),
        (SIDE_STAIRS_FRONT, vec![VOID_SIDE]),
        (SIDE_STAIRS_TOP, vec![VOID_BOTTOM]),
        (SIDE_STAIRS_BOTTOM, vec![GROUND_TOP]),
        (STAIRS_SUPPORT_SIDE, vec![STAIRS_SUPPORT_SIDE]),
        // (STAIRS_SUPPORT_BACK, vec![VOID_SIDE]),
        (STAIRS_SUPPORT_LEFT, vec![VOID_SIDE]),
        (STAIRS_SUPPORT_RIGHT, vec![VOID_SIDE]),
        (STAIRS_SUPPORT_FRONT, vec![VOID_SIDE]),
        (STAIRS_SUPPORT_BOTTOM, vec![GROUND_TOP]),
        (STAIRS_SUPPORT_TOP, vec![STAIRS_BOTTOM, SIDE_STAIRS_BOTTOM]),
        (
            PYRAMID_TOP_SIDE,
            vec![PYRAMID_TOP_SIDE, SIDE_STAIRS_BACK, STAIRS_BACK],
        ),
        // (PYRAMID_TOP_BACK, vec![VOID_SIDE]),
        // (PYRAMID_TOP_LEFT, vec![VOID_SIDE]),
        // (PYRAMID_TOP_RIGHT, vec![VOID_SIDE]),
        // (PYRAMID_TOP_FRONT, vec![VOID_SIDE]),
        (PYRAMID_TOP_BOTTOM, vec![VOID_TOP, GROUND_TOP]),
        (PYRAMID_TOP_TOP, vec![VOID_BOTTOM]),
    ];

    (models_asset_paths, models, sockets_connections)
}
