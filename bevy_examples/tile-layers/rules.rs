use bevy_ghx_proc_gen::proc_gen::{
    generator::node::{NodeModel, NodeRotation, Socket, SocketCollection, SocketsCartesian3D},
    grid::direction::{Cartesian3D, Direction},
};

const UP_AXIS: Direction = Direction::ZForward;

pub(crate) fn rules_and_assets() -> (
    Vec<Option<&'static str>>,
    Vec<NodeModel<Cartesian3D>>,
    SocketCollection,
) {
    let mut sockets = SocketCollection::new();

    let mut s = || -> Socket { sockets.create() };
    let (void, dirt) = (s(), s());
    let (layer_0_down, layer_0_up) = (s(), s());

    let (grass, void_and_grass, grass_and_void) = (s(), s(), s());
    let (layer_1_down, layer_1_up, grass_up) = (s(), s(), s());

    let yellow_grass_down = s();
    let (layer_2_down, layer_2_up) = (s(), s());

    let (water, void_and_water, water_and_void) = (s(), s(), s());
    let (layer_3_down, layer_3_up) = (s(), s());

    let green_grass_corner_out = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: void,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: void,
        y_neg: grass_and_void,
    }
    .new_model();
    let green_grass_corner_in = SocketsCartesian3D::Simple {
        x_pos: grass_and_void,
        x_neg: grass,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: grass,
        y_neg: void_and_grass,
    }
    .new_model();
    let green_grass_side = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: grass_and_void,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: void,
        y_neg: grass,
    }
    .new_model();

    let yellow_grass_corner_out = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: void,
        z_pos: layer_2_up,
        z_neg: yellow_grass_down,
        y_pos: void,
        y_neg: grass_and_void,
    }
    .new_model();
    let yellow_grass_corner_in = SocketsCartesian3D::Simple {
        x_pos: grass_and_void,
        x_neg: grass,
        z_pos: layer_2_up,
        z_neg: yellow_grass_down,
        y_pos: grass,
        y_neg: void_and_grass,
    }
    .new_model();
    let yellow_grass_side = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: grass_and_void,
        z_pos: layer_2_up,
        z_neg: yellow_grass_down,
        y_pos: void,
        y_neg: grass,
    }
    .new_model();

    const WATER_WEIGHT: f32 = 0.02;
    let water_corner_out = SocketsCartesian3D::Simple {
        x_pos: void_and_water,
        x_neg: void,
        z_pos: layer_3_up,
        z_neg: layer_3_down,
        y_pos: void,
        y_neg: water_and_void,
    }
    .new_model()
    .with_weight(WATER_WEIGHT);
    let water_corner_in = SocketsCartesian3D::Simple {
        x_pos: water_and_void,
        x_neg: water,
        z_pos: layer_3_up,
        z_neg: layer_3_down,
        y_pos: water,
        y_neg: void_and_water,
    }
    .new_model()
    .with_weight(WATER_WEIGHT);
    let water_side = SocketsCartesian3D::Simple {
        x_pos: void_and_water,
        x_neg: water_and_void,
        z_pos: layer_3_up,
        z_neg: layer_3_down,
        y_pos: void,
        y_neg: water,
    }
    .new_model()
    .with_weight(WATER_WEIGHT);

    let assets_and_models = vec![
        (
            Some("dirt"),
            SocketsCartesian3D::Simple {
                x_pos: dirt,
                x_neg: dirt,
                z_pos: layer_0_up,
                z_neg: layer_0_down,
                y_pos: dirt,
                y_neg: dirt,
            }
            .new_model()
            .with_weight(20.),
        ),
        // ------------------------------------------
        (
            None, // Layer 1 Void
            SocketsCartesian3D::Simple {
                x_pos: void,
                x_neg: void,
                z_pos: layer_1_up,
                z_neg: layer_1_down,
                y_pos: void,
                y_neg: void,
            }
            .new_model(),
        ),
        (
            Some("green_grass"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![grass],
                x_neg: vec![grass],
                z_pos: vec![layer_1_up, grass_up],
                z_neg: vec![layer_1_down],
                y_pos: vec![grass],
                y_neg: vec![grass],
            }
            .new_model()
            .with_weight(5.),
        ),
        // Here, we have different tiles asset for each rotation (grass blades are facing up), so we chose not to specify `with_all_rotations` but instead re-use a model definition by manually create rotatint it and creating different models.
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
        // ------------------------------------------
        (
            None, // Layer 2 Void
            SocketsCartesian3D::Simple {
                x_pos: void,
                x_neg: void,
                z_pos: layer_2_up,
                z_neg: layer_2_down,
                y_pos: void,
                y_neg: void,
            }
            .new_model(),
        ),
        (
            Some("yellow_grass"),
            SocketsCartesian3D::Simple {
                x_pos: grass,
                x_neg: grass,
                z_pos: layer_2_up,
                z_neg: layer_2_down,
                y_pos: grass,
                y_neg: grass,
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
        // ------------------------------------------
        (
            None, // Layer 3 Void
            SocketsCartesian3D::Simple {
                x_pos: void,
                x_neg: void,
                z_pos: layer_3_up,
                z_neg: layer_3_down,
                y_pos: void,
                y_neg: void,
            }
            .new_model(),
        ),
        (
            Some("water"),
            SocketsCartesian3D::Simple {
                x_pos: water,
                x_neg: water,
                z_pos: layer_3_up,
                z_neg: layer_3_down,
                y_pos: water,
                y_neg: water,
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
    sockets
        .add_connections(vec![
            (dirt, vec![dirt]),
            (void, vec![void]),
            (grass, vec![grass]),
            (void_and_grass, vec![grass_and_void]),
            (water, vec![water]),
            (water_and_void, vec![void_and_water]),
        ])
        .add_rotated_connection(layer_0_up, vec![layer_1_down])
        .add_rotated_connection(layer_1_up, vec![layer_2_down])
        .add_rotated_connection(layer_2_up, vec![layer_3_down])
        .add_rotated_connection(yellow_grass_down, vec![grass_up]);

    (
        assets_and_models.iter().map(|t| t.0).collect(),
        assets_and_models
            .iter()
            .map(|t| t.1.clone().with_name(t.0.unwrap_or("void")))
            .collect(),
        sockets,
    )
}
