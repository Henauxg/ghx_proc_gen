use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        node::{NodeModel, NodeRotation, SocketId, SocketsCartesian3D},
        rules::SocketConnections,
    },
    grid::direction::{Cartesian3D, Direction},
};

const UP_AXIS: Direction = Direction::ZForward;

const VOID: SocketId = 0;
const DIRT: SocketId = 1;
const L0_DOWN: SocketId = 2;
const L0_UP: SocketId = 3;

const GRASS: SocketId = 10;
const VOID_AND_GRASS: SocketId = 11;
const GRASS_AND_VOID: SocketId = 12;
const L1_UP: SocketId = 13;
const L1_DOWN: SocketId = 14;
const GRASS_UP: SocketId = 15;

const YELLOW_GRASS_DOWN: SocketId = 20;
const L2_UP: SocketId = 23;
const L2_DOWN: SocketId = 24;

const WATER: SocketId = 30;
const VOID_AND_WATER: SocketId = 31;
const WATER_AND_VOID: SocketId = 32;
const L3_UP: SocketId = 33;
const L3_DOWN: SocketId = 34;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian3D>>,
    Vec<SocketConnections>,
) {
    let green_grass_corner_out = SocketsCartesian3D::Simple {
        x_pos: VOID_AND_GRASS,
        x_neg: VOID,
        z_pos: L1_UP,
        z_neg: L1_DOWN,
        y_pos: VOID,
        y_neg: GRASS_AND_VOID,
    }
    .new_model();
    let green_grass_corner_in = SocketsCartesian3D::Simple {
        x_pos: GRASS_AND_VOID,
        x_neg: GRASS,
        z_pos: L1_UP,
        z_neg: L1_DOWN,
        y_pos: GRASS,
        y_neg: VOID_AND_GRASS,
    }
    .new_model();
    let green_grass_side = SocketsCartesian3D::Simple {
        x_pos: VOID_AND_GRASS,
        x_neg: GRASS_AND_VOID,
        z_pos: L1_UP,
        z_neg: L1_DOWN,
        y_pos: VOID,
        y_neg: GRASS,
    }
    .new_model();

    let yellow_grass_corner_out = SocketsCartesian3D::Simple {
        x_pos: VOID_AND_GRASS,
        x_neg: VOID,
        z_pos: L2_UP,
        z_neg: YELLOW_GRASS_DOWN,
        y_pos: VOID,
        y_neg: GRASS_AND_VOID,
    }
    .new_model();
    let yellow_grass_corner_in = SocketsCartesian3D::Simple {
        x_pos: GRASS_AND_VOID,
        x_neg: GRASS,
        z_pos: L2_UP,
        z_neg: YELLOW_GRASS_DOWN,
        y_pos: GRASS,
        y_neg: VOID_AND_GRASS,
    }
    .new_model();
    let yellow_grass_side = SocketsCartesian3D::Simple {
        x_pos: VOID_AND_GRASS,
        x_neg: GRASS_AND_VOID,
        z_pos: L2_UP,
        z_neg: YELLOW_GRASS_DOWN,
        y_pos: VOID,
        y_neg: GRASS,
    }
    .new_model();

    const WATER_WEIGHT: f32 = 0.02;
    let water_corner_out = SocketsCartesian3D::Simple {
        x_pos: VOID_AND_WATER,
        x_neg: VOID,
        z_pos: L3_UP,
        z_neg: L3_DOWN,
        y_pos: VOID,
        y_neg: WATER_AND_VOID,
    }
    .new_model()
    .with_weight(WATER_WEIGHT);
    let water_corner_in = SocketsCartesian3D::Simple {
        x_pos: WATER_AND_VOID,
        x_neg: WATER,
        z_pos: L3_UP,
        z_neg: L3_DOWN,
        y_pos: WATER,
        y_neg: VOID_AND_WATER,
    }
    .new_model()
    .with_weight(WATER_WEIGHT);
    let water_side = SocketsCartesian3D::Simple {
        x_pos: VOID_AND_WATER,
        x_neg: WATER_AND_VOID,
        z_pos: L3_UP,
        z_neg: L3_DOWN,
        y_pos: VOID,
        y_neg: WATER,
    }
    .new_model()
    .with_weight(WATER_WEIGHT);

    let assets_and_models = vec![
        (
            Some("dirt"),
            SocketsCartesian3D::Simple {
                x_pos: DIRT,
                x_neg: DIRT,
                z_pos: L0_UP,
                z_neg: L0_DOWN,
                y_pos: DIRT,
                y_neg: DIRT,
            }
            .new_model(),
        ),
        (
            None, // L1 Void
            SocketsCartesian3D::Simple {
                x_pos: VOID,
                x_neg: VOID,
                z_pos: L1_UP,
                z_neg: L1_DOWN,
                y_pos: VOID,
                y_neg: VOID,
            }
            .new_model(),
        ),
        (
            Some("green_grass"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![GRASS],
                x_neg: vec![GRASS],
                z_pos: vec![L1_UP, GRASS_UP],
                z_neg: vec![L1_DOWN],
                y_pos: vec![GRASS],
                y_neg: vec![GRASS],
            }
            .new_model()
            .with_weight(5.),
        ),
        (
            Some("green_grass_corner_out_tl"),
            green_grass_corner_out.clone(),
        ),
        (
            Some("green_grass_corner_out_bl"),
            green_grass_corner_out.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("green_grass_corner_out_br"),
            green_grass_corner_out.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("green_grass_corner_out_tr"),
            green_grass_corner_out.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (
            Some("green_grass_corner_in_tl"),
            green_grass_corner_in.clone(),
        ),
        (
            Some("green_grass_corner_in_bl"),
            green_grass_corner_in.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("green_grass_corner_in_br"),
            green_grass_corner_in.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("green_grass_corner_in_tr"),
            green_grass_corner_in.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (Some("green_grass_side_t"), green_grass_side.clone()),
        (
            Some("green_grass_side_l"),
            green_grass_side.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("green_grass_side_b"),
            green_grass_side.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("green_grass_side_r"),
            green_grass_side.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (
            None, // L2 Void
            SocketsCartesian3D::Simple {
                x_pos: VOID,
                x_neg: VOID,
                z_pos: L2_UP,
                z_neg: L2_DOWN,
                y_pos: VOID,
                y_neg: VOID,
            }
            .new_model(),
        ),
        (
            Some("yellow_grass"),
            SocketsCartesian3D::Simple {
                x_pos: GRASS,
                x_neg: GRASS,
                z_pos: L2_UP,
                z_neg: L2_DOWN,
                y_pos: GRASS,
                y_neg: GRASS,
            }
            .new_model()
            .with_weight(1.),
        ),
        (
            Some("yellow_grass_corner_out_tl"),
            yellow_grass_corner_out.clone(),
        ),
        (
            Some("yellow_grass_corner_out_bl"),
            yellow_grass_corner_out.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("yellow_grass_corner_out_br"),
            yellow_grass_corner_out.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("yellow_grass_corner_out_tr"),
            yellow_grass_corner_out.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (
            Some("yellow_grass_corner_in_tl"),
            yellow_grass_corner_in.clone(),
        ),
        (
            Some("yellow_grass_corner_in_bl"),
            yellow_grass_corner_in.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("yellow_grass_corner_in_br"),
            yellow_grass_corner_in.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("yellow_grass_corner_in_tr"),
            yellow_grass_corner_in.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (Some("yellow_grass_side_t"), yellow_grass_side.clone()),
        (
            Some("yellow_grass_side_l"),
            yellow_grass_side.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("yellow_grass_side_b"),
            yellow_grass_side.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("yellow_grass_side_r"),
            yellow_grass_side.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (
            None, // L3 Void
            SocketsCartesian3D::Simple {
                x_pos: VOID,
                x_neg: VOID,
                z_pos: L3_UP,
                z_neg: L3_DOWN,
                y_pos: VOID,
                y_neg: VOID,
            }
            .new_model(),
        ),
        (
            Some("water"),
            SocketsCartesian3D::Simple {
                x_pos: WATER,
                x_neg: WATER,
                z_pos: L3_UP,
                z_neg: L3_DOWN,
                y_pos: WATER,
                y_neg: WATER,
            }
            .new_model()
            .with_weight(10. * WATER_WEIGHT),
        ),
        (Some("water_corner_out_tl"), water_corner_out.clone()),
        (
            Some("water_corner_out_bl"),
            water_corner_out.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("water_corner_out_br"),
            water_corner_out.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("water_corner_out_tr"),
            water_corner_out.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (Some("water_corner_in_tl"), water_corner_in.clone()),
        (
            Some("water_corner_in_bl"),
            water_corner_in.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("water_corner_in_br"),
            water_corner_in.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("water_corner_in_tr"),
            water_corner_in.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
        (Some("water_side_t"), water_side.clone()),
        (
            Some("water_side_l"),
            water_side.rotated(NodeRotation::Rot90, UP_AXIS),
        ),
        (
            Some("water_side_b"),
            water_side.rotated(NodeRotation::Rot180, UP_AXIS),
        ),
        (
            Some("water_side_r"),
            water_side.rotated(NodeRotation::Rot270, UP_AXIS),
        ),
    ];
    let sockets_connections = vec![
        (DIRT, vec![DIRT]),
        (VOID, vec![VOID]),
        (L0_UP, vec![L1_DOWN]),
        (L1_UP, vec![L2_DOWN]),
        (L2_UP, vec![L3_DOWN]),
        (GRASS, vec![GRASS]),
        (VOID_AND_GRASS, vec![GRASS_AND_VOID]),
        (YELLOW_GRASS_DOWN, vec![GRASS_UP]),
        (WATER, vec![WATER]),
        (WATER_AND_VOID, vec![VOID_AND_WATER]),
    ];
    (
        assets_and_models.iter().map(|t| t.0).collect(),
        assets_and_models.iter().map(|t| t.1.clone()).collect(),
        sockets_connections,
    )
}
