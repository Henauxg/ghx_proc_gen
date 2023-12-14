use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        node::{NodeModel, NodeRotation, SocketId, SocketsCartesian2D},
        rules::SocketConnections,
    },
    grid::direction::Cartesian2D,
};

const WATER: SocketId = 0;

const BEACHSHORE: SocketId = 1;
const BEACHSHORE_WATER: SocketId = 2;
const WATER_BEACHSHORE: SocketId = 3;

const BEACH: SocketId = 4;
const BEACH_SIDE: SocketId = 5;
const LAND: SocketId = 6;

const GRASS: SocketId = 10;
const GRASS_OTHER: SocketId = 11;
const OTHER_GRASS: SocketId = 12;
const GRASS_BORDER: SocketId = 13;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian2D>>,
    Vec<SocketConnections>,
) {
    // Define some models here to be cloned/rotated manually
    let grass_corner_bl = SocketsCartesian2D::Simple {
        x_pos: GRASS_OTHER,
        x_neg: GRASS_BORDER,
        y_pos: OTHER_GRASS,
        y_neg: GRASS_BORDER,
    }
    .new_model();
    let grass_side_b = SocketsCartesian2D::Simple {
        x_pos: GRASS_OTHER,
        x_neg: OTHER_GRASS,
        y_pos: GRASS,
        y_neg: GRASS_BORDER,
    }
    .new_model();

    let assets_and_models = vec![
        (Some("water"), SocketsCartesian2D::Mono(WATER).new_model()),
        (
            Some("beachshore"),
            SocketsCartesian2D::Mono(BEACHSHORE).new_model(),
        ),
        (
            Some("water_beachshore_1"),
            SocketsCartesian2D::Simple {
                x_pos: WATER_BEACHSHORE,
                x_neg: BEACHSHORE_WATER,
                y_pos: WATER,
                y_neg: BEACHSHORE,
            }
            .new_model()
            .with_all_rotations(),
        ),
        (
            Some("water_beachshore_2"),
            SocketsCartesian2D::Simple {
                x_pos: WATER,
                x_neg: BEACHSHORE,
                y_pos: BEACHSHORE_WATER,
                y_neg: WATER_BEACHSHORE,
            }
            .new_model()
            .with_all_rotations(),
        ),
        (
            Some("water_beachshore_corner_1"),
            SocketsCartesian2D::Simple {
                x_pos: WATER_BEACHSHORE,
                x_neg: WATER,
                y_pos: WATER,
                y_neg: BEACHSHORE_WATER,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("waters_beachshore_corner_2"),
            SocketsCartesian2D::Simple {
                x_pos: WATER,
                x_neg: BEACHSHORE_WATER,
                y_pos: WATER,
                y_neg: WATER_BEACHSHORE,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("water_beachshore_double_corner"),
            SocketsCartesian2D::Simple {
                x_pos: WATER_BEACHSHORE,
                x_neg: WATER_BEACHSHORE,
                y_pos: BEACHSHORE_WATER,
                y_neg: BEACHSHORE_WATER,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.05),
        ),
        (
            Some("beachshore_water_corner_1"),
            SocketsCartesian2D::Simple {
                x_pos: BEACHSHORE_WATER,
                x_neg: BEACHSHORE,
                y_pos: BEACHSHORE,
                y_neg: WATER_BEACHSHORE,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("beachshore_water_corner_2"),
            SocketsCartesian2D::Simple {
                x_pos: BEACHSHORE,
                x_neg: WATER_BEACHSHORE,
                y_pos: BEACHSHORE,
                y_neg: BEACHSHORE_WATER,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("beach_corner_1"),
            SocketsCartesian2D::Simple {
                x_pos: BEACH_SIDE,
                x_neg: BEACH,
                y_pos: BEACH,
                y_neg: BEACH_SIDE,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("beach_side_1"),
            SocketsCartesian2D::Simple {
                x_pos: BEACH_SIDE,
                x_neg: BEACH_SIDE,
                y_pos: LAND,
                y_neg: BEACH,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("grass"),
            SocketsCartesian2D::Simple {
                x_pos: GRASS,
                x_neg: GRASS,
                y_pos: GRASS,
                y_neg: GRASS,
            }
            .new_model(),
        ),
        // Here, we have different tiles asset for each rotation (grass blades are facing up), so we chose not to specify `with_all_rotations` but instead manually create different models, but still re-use a model definition.
        (Some("grass_corner_bl"), grass_corner_bl.clone()),
        (
            Some("grass_corner_br"),
            grass_corner_bl.rotated(NodeRotation::Rot90),
        ),
        (
            Some("grass_corner_tr"),
            grass_corner_bl.rotated(NodeRotation::Rot180),
        ),
        (
            Some("grass_corner_tl"),
            grass_corner_bl.rotated(NodeRotation::Rot270),
        ),
        (Some("grass_side_b"), grass_side_b.clone()),
        (
            Some("grass_side_r"),
            grass_side_b.rotated(NodeRotation::Rot90),
        ),
        (
            Some("grass_side_t"),
            grass_side_b.rotated(NodeRotation::Rot180),
        ),
        (
            Some("grass_side_l"),
            grass_side_b.rotated(NodeRotation::Rot270),
        ),
    ];
    let sockets_connections = vec![
        (WATER, vec![WATER]),
        // (BEACHSHORE, vec![BEACHSHORE]),
        (BEACHSHORE_WATER, vec![WATER_BEACHSHORE]),
        (BEACHSHORE, vec![BEACH]),
        (BEACH_SIDE, vec![BEACH_SIDE]),
        (GRASS, vec![GRASS]),
        (OTHER_GRASS, vec![GRASS_OTHER]),
        (GRASS_BORDER, vec![LAND]),
    ];
    (
        assets_and_models.iter().map(|t| t.0).collect(),
        assets_and_models.iter().map(|t| t.1.clone()).collect(),
        sockets_connections,
    )
}
