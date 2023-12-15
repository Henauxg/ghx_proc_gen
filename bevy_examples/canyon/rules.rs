use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        node::{NodeModel, SocketId, SocketsCartesian3D},
        rules::SocketConnections,
    },
    grid::direction::Cartesian3D,
};

const VOID: SocketId = 0;
const VOID_TOP: SocketId = 1;
const VOID_BOTTOM: SocketId = 2;

const WATER: SocketId = 10;
const WATER_BORDER: SocketId = 11;
const WATER_TOP: SocketId = 12;
const WATER_BOTTOM: SocketId = 13;

const SAND: SocketId = 20;
const SAND_BORDER: SocketId = 21;
const SAND_TOP: SocketId = 22;
const SAND_BOTTOM: SocketId = 23;

const ROCK: SocketId = 30;
const ROCK_BORDER: SocketId = 31;
const ROCK_BORDER_TOP: SocketId = 32;
const ROCK_BORDER_BOTTOM: SocketId = 33;
const ROCK_TOP: SocketId = 34;
const ROCK_BOTTOM: SocketId = 35;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian3D>>,
    Vec<SocketConnections>,
) {
    let assets_and_models = vec![
        (
            None,
            SocketsCartesian3D::Simple {
                x_pos: VOID,
                x_neg: VOID,
                z_pos: VOID,
                z_neg: VOID,
                y_pos: VOID_TOP,
                y_neg: VOID_BOTTOM,
            }
            .new_model(),
        ),
        (
            Some("water"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![WATER],
                x_neg: vec![WATER, WATER_BORDER],
                z_pos: vec![WATER],
                z_neg: vec![WATER, WATER_BORDER],
                y_pos: vec![WATER_TOP],
                y_neg: vec![WATER_BOTTOM],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(100.25),
        ),
        (
            Some("sand"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![SAND],
                x_neg: vec![SAND, SAND_BORDER],
                z_pos: vec![SAND],
                z_neg: vec![SAND, SAND_BORDER],
                y_pos: vec![SAND_TOP],
                y_neg: vec![SAND_BOTTOM],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(100.25),
        ),
        (
            Some("rock_corner_in"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![ROCK_BORDER],
                x_neg: vec![ROCK], // TODO May use OTHER_TO_ROCK/ROCK_TO_OTHER
                z_pos: vec![ROCK_BORDER],
                z_neg: vec![ROCK],
                y_pos: vec![ROCK_BORDER_TOP],
                y_neg: vec![ROCK_BORDER_BOTTOM],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("rock_side"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![ROCK],
                x_neg: vec![ROCK],
                z_pos: vec![ROCK],
                z_neg: vec![ROCK_BORDER],
                y_pos: vec![ROCK_BORDER_TOP],
                y_neg: vec![ROCK_BORDER_BOTTOM],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            Some("rock_side"), // rock
            SocketsCartesian3D::Multiple {
                x_pos: vec![ROCK],
                x_neg: vec![ROCK],
                z_pos: vec![ROCK],
                z_neg: vec![ROCK],
                y_pos: vec![ROCK_TOP],
                y_neg: vec![ROCK_BOTTOM],
            }
            .new_model(), // .with_all_rotations()
                               // .with_weight(0.25),
        ),
    ];
    let sockets_connections = vec![
        (VOID, vec![VOID]),
        (VOID_BOTTOM, vec![VOID_TOP]),
        (WATER, vec![WATER]),
        (WATER_TOP, vec![VOID_BOTTOM]),
        (SAND, vec![SAND]),
        (SAND_TOP, vec![VOID_BOTTOM]),
        (SAND_BORDER, vec![WATER_BORDER]),
        (ROCK_BORDER, vec![WATER_BORDER, SAND_BORDER, VOID]), // TMP VOID
        (ROCK_BORDER_TOP, vec![VOID_BOTTOM]),
        (ROCK, vec![ROCK]),
        (ROCK_TOP, vec![ROCK_BOTTOM, ROCK_BORDER_BOTTOM, VOID_BOTTOM]),
    ];
    (
        assets_and_models.iter().map(|t| t.0).collect(),
        assets_and_models.iter().map(|t| t.1.clone()).collect(),
        sockets_connections,
    )
}
