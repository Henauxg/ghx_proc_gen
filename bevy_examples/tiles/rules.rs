use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        node::{NodeModel, SocketId, SocketsCartesian2D},
        rules::SocketConnections,
    },
    grid::direction::Cartesian2D,
};

const WATER: SocketId = 0;
const SAND: SocketId = 1;
const SAND_WATER: SocketId = 2;
const WATER_SAND: SocketId = 3;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian2D>>,
    Vec<SocketConnections>,
) {
    let assets_and_models = vec![
        (Some("water"), SocketsCartesian2D::Mono(WATER).new_model()),
        (Some("sand"), SocketsCartesian2D::Mono(SAND).new_model()),
        (
            Some("water_sand_1"),
            SocketsCartesian2D::Simple {
                x_pos: WATER_SAND,
                x_neg: SAND_WATER,
                y_pos: WATER,
                y_neg: SAND,
            }
            .new_model()
            .with_all_rotations(),
        ),
        (
            Some("water_sand_2"),
            SocketsCartesian2D::Simple {
                x_pos: WATER,
                x_neg: SAND,
                y_pos: SAND_WATER,
                y_neg: WATER_SAND,
            }
            .new_model()
            .with_all_rotations(),
        ),
        (
            Some("water_sand_corner_1"),
            SocketsCartesian2D::Simple {
                x_pos: WATER_SAND,
                x_neg: WATER,
                y_pos: WATER,
                y_neg: SAND_WATER,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("waters_and_corner_2"),
            SocketsCartesian2D::Simple {
                x_pos: WATER,
                x_neg: SAND_WATER,
                y_pos: WATER,
                y_neg: WATER_SAND,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("water_sand_double_corner"),
            SocketsCartesian2D::Simple {
                x_pos: WATER_SAND,
                x_neg: WATER_SAND,
                y_pos: SAND_WATER,
                y_neg: SAND_WATER,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("sand_water_corner_1"),
            SocketsCartesian2D::Simple {
                x_pos: SAND_WATER,
                x_neg: SAND,
                y_pos: SAND,
                y_neg: WATER_SAND,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("sand_water_corner_2"),
            SocketsCartesian2D::Simple {
                x_pos: SAND,
                x_neg: WATER_SAND,
                y_pos: SAND,
                y_neg: SAND_WATER,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
    ];
    let sockets_connections = vec![
        (WATER, vec![WATER]),
        (SAND, vec![SAND]),
        (SAND_WATER, vec![WATER_SAND]),
    ];
    (
        assets_and_models.iter().map(|t| t.0).collect(),
        assets_and_models.iter().map(|t| t.1.clone()).collect(),
        sockets_connections,
    )
}
