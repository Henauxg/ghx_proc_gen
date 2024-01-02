use bevy_examples::AssetDef;
use bevy_ghx_proc_gen::proc_gen::{
    generator::node::{NodeModel, NodeRotation, Socket, SocketCollection, SocketsCartesian3D},
    grid::direction::Cartesian3D,
};

use crate::SEE_VOID_NODES;

pub(crate) fn rules_and_assets() -> (
    Vec<Vec<AssetDef>>,
    Vec<NodeModel<Cartesian3D>>,
    SocketCollection,
) {
    let mut sockets = SocketCollection::new();

    let mut s = || -> Socket { sockets.create() };
    let (void, void_top, void_bottom) = (s(), s(), s());
    let (water, water_border, water_top, water_bottom) = (s(), s(), s(), s());
    let (sand, sand_border, sand_top, sand_bottom) = (s(), s(), s(), s());
    let (ground_rock_border, ground_rock_border_top, ground_rock_border_bottom) = (s(), s(), s());
    let (ground_rock_to_other, other_to_ground_rock) = (s(), s());
    let (rock, rock_top, rock_bottom) = (s(), s(), s());
    let (rock_border, rock_border_top, rock_border_bottom) = (s(), s(), s());
    let (rock_to_other, other_to_rock) = (s(), s());
    let (bridge, bridge_side, bridge_top, bridge_bottom) = (s(), s(), s(), s());
    let (bridge_start_in, bridge_start_out, bridge_start_bottom) = (s(), s(), s());
    let (platform_side, platform_back, platform_top, platform_bottom) = (s(), s(), s(), s());
    let (
        platform_support_top,
        platform_support_bottom,
        platform_support_long_top,
        platform_support_long_bottom,
    ) = (s(), s(), s(), s());
    let (cactus_border, cactus_top, cactus_bottom) = (s(), s(), s());

    let asset = |str| -> Vec<AssetDef> { vec![AssetDef::new(str)] };

    let assets_and_models = vec![
        (
            match SEE_VOID_NODES {
                true => asset("void"),
                false => vec![],
            },
            SocketsCartesian3D::Simple {
                x_pos: void,
                x_neg: void,
                z_pos: void,
                z_neg: void,
                y_pos: void_top,
                y_neg: void_bottom,
            }
            .new_model()
            .with_weight(10.),
        ),
        (
            asset("water_poly"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![water],
                x_neg: vec![water, water_border],
                z_pos: vec![water],
                z_neg: vec![water, water_border],
                y_pos: vec![water_top],
                y_neg: vec![water_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(10.25),
        ),
        (
            asset("sand"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![sand],
                x_neg: vec![sand, sand_border],
                z_pos: vec![sand],
                z_neg: vec![sand, sand_border],
                y_pos: vec![sand_top],
                y_neg: vec![sand_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(10.25),
        ),
        (
            asset("cactus"),
            SocketsCartesian3D::Simple {
                x_pos: cactus_border,
                x_neg: cactus_border,
                z_pos: cactus_border,
                z_neg: cactus_border,
                y_pos: cactus_top,
                y_neg: cactus_bottom,
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.25),
        ),
        (
            asset("ground_rock_corner_in"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![ground_rock_border],
                x_neg: vec![other_to_ground_rock],
                z_pos: vec![ground_rock_border],
                z_neg: vec![ground_rock_to_other],
                y_pos: vec![ground_rock_border_top],
                y_neg: vec![ground_rock_border_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.5),
        ),
        (
            asset("ground_rock_side"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![ground_rock_border],
                x_neg: vec![rock],
                z_pos: vec![other_to_ground_rock],
                z_neg: vec![ground_rock_to_other],
                y_pos: vec![ground_rock_border_top],
                y_neg: vec![ground_rock_border_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.5),
        ),
        (
            asset("rock_corner_in_1"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![rock_border],
                x_neg: vec![other_to_rock],
                z_pos: vec![rock_border],
                z_neg: vec![rock_to_other],
                y_pos: vec![rock_border_top],
                y_neg: vec![rock_border_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.05),
        ),
        (
            asset("rock_corner_in_2"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![rock_border],
                x_neg: vec![other_to_rock],
                z_pos: vec![rock_border],
                z_neg: vec![rock_to_other],
                y_pos: vec![rock_border_top],
                y_neg: vec![rock_border_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.05),
        ),
        (
            asset("rock_side_1"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![rock_border],
                x_neg: vec![rock],
                z_pos: vec![other_to_rock],
                z_neg: vec![rock_to_other],
                y_pos: vec![rock_border_top],
                y_neg: vec![rock_border_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.05),
        ),
        (
            asset("rock"), // rock
            SocketsCartesian3D::Multiple {
                x_pos: vec![rock],
                x_neg: vec![rock],
                z_pos: vec![rock],
                z_neg: vec![rock],
                y_pos: vec![rock_top],
                y_neg: vec![rock_bottom],
            }
            .new_model()
            .with_weight(0.05),
        ),
        (
            asset("bridge_start"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![bridge_side],
                x_neg: vec![bridge_side],
                z_pos: vec![bridge_start_out],
                z_neg: vec![bridge_start_in],
                y_pos: vec![bridge_top],
                y_neg: vec![bridge_start_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.05),
        ),
        (
            asset("bridge"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![bridge_side],
                x_neg: vec![bridge_side],
                z_pos: vec![bridge],
                z_neg: vec![bridge],
                y_pos: vec![bridge_top],
                y_neg: vec![bridge_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.05),
        ),
        (
            asset("platform"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![platform_side],
                x_neg: vec![platform_side],
                z_pos: vec![bridge_start_in],
                z_neg: vec![platform_back],
                y_pos: vec![platform_top],
                y_neg: vec![platform_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.01),
        ),
        (
            asset("platform_support"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![platform_side],
                x_neg: vec![platform_side],
                z_pos: vec![platform_side],
                z_neg: vec![platform_side],
                y_pos: vec![platform_support_top],
                y_neg: vec![platform_support_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.000001),
        ),
        (
            asset("platform_support_long"),
            SocketsCartesian3D::Multiple {
                x_pos: vec![platform_side],
                x_neg: vec![platform_side],
                z_pos: vec![platform_side],
                z_neg: vec![platform_side],
                y_pos: vec![platform_support_long_top],
                y_neg: vec![platform_support_long_bottom],
            }
            .new_model()
            .with_all_rotations()
            .with_weight(0.000001),
        ),
    ];
    sockets
        // Void
        .add_connection(void, vec![void])
        .add_rotated_connection(void_bottom, vec![void_top])
        // Water & sand
        .add_connection(water, vec![water])
        .add_rotated_connection(water_top, vec![void_bottom])
        .add_connection(sand, vec![sand])
        .add_connection(sand_border, vec![water_border])
        .add_rotated_connection(sand_top, vec![void_bottom])
        // Rocks
        .add_connections(vec![
            (ground_rock_border, vec![water, sand]),
            (ground_rock_to_other, vec![other_to_ground_rock]),
        ])
        .add_rotated_connection(
            ground_rock_border_top,
            vec![void_bottom, rock_border_bottom],
        )
        .add_connections(vec![
            (rock, vec![rock]),
            (rock_border, vec![void]),
            (rock_to_other, vec![other_to_rock]),
        ])
        .add_rotated_connection(rock_border_top, vec![void_bottom, rock_border_bottom])
        .add_rotated_connection(rock_top, vec![rock_bottom, rock_border_bottom, void_bottom])
        // Bridges
        .add_connections(vec![
            (bridge, vec![bridge]),
            (bridge_side, vec![void, rock_border]),
            (bridge_start_out, vec![void, rock_border]),
            (bridge_start_in, vec![bridge]),
        ])
        .add_rotated_connection(bridge_top, vec![void_bottom, bridge_bottom])
        .add_rotated_connection(bridge_bottom, vec![void_top, sand_top, water_top])
        // A bridge start model should face outwards from a rock.
        .add_constrained_rotated_connection(
            bridge_start_bottom,
            vec![NodeRotation::Rot180, NodeRotation::Rot270],
            vec![rock_border_top, ground_rock_border_top],
        )
        // Platforms
        .add_connections(vec![
            (platform_side, vec![void, rock_border, ground_rock_border]),
            (platform_back, vec![void]),
        ])
        .add_rotated_connection(platform_top, vec![void])
        // Keep the ladders and the wood beams aligned
        .add_constrained_rotated_connection(
            platform_bottom,
            vec![NodeRotation::Rot0],
            vec![platform_support_top, platform_support_long_top],
        )
        .add_constrained_rotated_connection(
            platform_support_bottom,
            vec![NodeRotation::Rot0],
            vec![platform_support_top, platform_support_long_top],
        )
        .add_rotated_connection(platform_support_bottom, vec![rock_top])
        .add_rotated_connection(platform_support_long_bottom, vec![sand_top, water_top])
        // Cactuses
        .add_connection(
            cactus_border,
            vec![void, rock_border, bridge_side, platform_side],
        )
        .add_rotated_connections(vec![
            (cactus_bottom, vec![sand_top]),
            (cactus_top, vec![void_bottom, bridge_bottom]),
        ]);
    (
        // Assets
        assets_and_models.iter().map(|t| t.0.clone()).collect(),
        // Node models
        assets_and_models
            .iter()
            .map(|t| {
                t.1.clone()
                    .with_name(t.0.first().unwrap_or(&AssetDef::new("void")).path())
            })
            .collect(),
        sockets,
    )
}
