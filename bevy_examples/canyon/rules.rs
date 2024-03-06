use bevy::{ecs::component::Component, math::Vec3};
use bevy_examples::utils::AssetDef;
use bevy_ghx_grid::ghx_grid::{coordinate_system::Cartesian3D, direction::GridDelta};
use bevy_ghx_proc_gen::{
    gen::assets::ComponentSpawner,
    proc_gen::generator::{
        model::{ModelCollection, ModelInstance, ModelRotation},
        socket::{Socket, SocketCollection, SocketsCartesian3D},
    },
};

use crate::{BLOCK_SIZE, SEE_VOID_NODES};

pub(crate) fn rules_and_assets() -> (
    ModelInstance,
    ModelInstance,
    ModelInstance,
    ModelInstance,
    Vec<Vec<AssetDef<CustomComponents>>>,
    ModelCollection<Cartesian3D>,
    SocketCollection,
) {
    let mut sockets = SocketCollection::new();

    // Create our sockets
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
    let (sand_prop_border, sand_prop_top, sand_prop_bottom) = (s(), s(), s());
    let (
        windmill_side,
        windmill_base_top,
        windmill_base_bottom,
        windmill_cap_top,
        windmill_cap_bottom,
    ) = (s(), s(), s(), s(), s());

    // Create our models. We declare our assets at the same time for clarity (index of the model matches the index of the assets to spawn).
    let mut models = ModelCollection::<Cartesian3D>::new();
    let mut assets = Vec::new();

    // Utility functions to declare assets & models
    let asset = |str| -> Vec<AssetDef<CustomComponents>> { vec![AssetDef::new(str)] };

    let void_instance = models
        .create(SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: void,
            z_pos: void,
            z_neg: void,
            y_pos: void_top,
            y_neg: void_bottom,
        })
        .with_weight(10.)
        .instance();
    assets.push(match SEE_VOID_NODES {
        true => asset("void"),
        false => vec![],
    });

    let water_instance = models
        .create(SocketsCartesian3D::Multiple {
            x_pos: vec![water],
            x_neg: vec![water, water_border],
            z_pos: vec![water],
            z_neg: vec![water, water_border],
            y_pos: vec![water_top],
            y_neg: vec![water_bottom],
        })
        .with_all_rotations()
        .with_weight(350.0)
        .instance();
    assets.push(asset("water_poly"));

    let sand_instance = models
        .create(SocketsCartesian3D::Multiple {
            x_pos: vec![sand],
            x_neg: vec![sand, sand_border],
            z_pos: vec![sand],
            z_neg: vec![sand, sand_border],
            y_pos: vec![sand_top],
            y_neg: vec![sand_bottom],
        })
        .with_weight(5.0)
        .instance();
    assets.push(asset("sand"));

    const GROUND_ROCKS_WEIGHT: f32 = 0.5;
    const ROCKS_WEIGHT: f32 = 0.05;
    // Here we define a model template that we'll reuse multiple times
    let rock_corner = SocketsCartesian3D::Simple {
        x_pos: rock_border,
        x_neg: other_to_rock,
        z_pos: rock_border,
        z_neg: rock_to_other,
        y_pos: rock_border_top,
        y_neg: rock_border_bottom,
    }
    .to_template()
    .with_all_rotations()
    .with_weight(ROCKS_WEIGHT);

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: ground_rock_border,
            x_neg: other_to_ground_rock,
            z_pos: ground_rock_border,
            z_neg: ground_rock_to_other,
            y_pos: ground_rock_border_top,
            y_neg: ground_rock_border_bottom,
        })
        .with_all_rotations()
        .with_weight(GROUND_ROCKS_WEIGHT);
    assets.push(asset("ground_rock_corner_in"));

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: ground_rock_border,
            x_neg: rock,
            z_pos: other_to_ground_rock,
            z_neg: ground_rock_to_other,
            y_pos: ground_rock_border_top,
            y_neg: ground_rock_border_bottom,
        })
        .with_all_rotations()
        .with_weight(GROUND_ROCKS_WEIGHT);
    assets.push(asset("ground_rock_side"));

    // Here we reuse the same model to create variations. (We could also have 1 model, and 2 assets, with the spawner picking one of the assets at random)
    models.create(rock_corner.clone());
    assets.push(asset("rock_corner_in_1"));

    models.create(rock_corner.clone());
    assets.push(asset("rock_corner_in_2"));

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: rock_border,
            x_neg: rock,
            z_pos: other_to_rock,
            z_neg: rock_to_other,
            y_pos: rock_border_top,
            y_neg: rock_border_bottom,
        })
        .with_all_rotations()
        .with_weight(ROCKS_WEIGHT);
    assets.push(asset("rock_side_1"));

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: rock,
            x_neg: rock,
            z_pos: rock,
            z_neg: rock,
            y_pos: rock_top,
            y_neg: rock_bottom,
        })
        .with_weight(ROCKS_WEIGHT);
    assets.push(asset("rock"));

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: bridge_side,
            x_neg: bridge_side,
            z_pos: bridge_start_out,
            z_neg: bridge_start_in,
            y_pos: bridge_top,
            y_neg: bridge_start_bottom,
        })
        .with_all_rotations()
        .with_weight(0.05);
    assets.push(asset("bridge_start"));

    let bridge_instance = models
        .create(SocketsCartesian3D::Simple {
            x_pos: bridge_side,
            x_neg: bridge_side,
            z_pos: bridge,
            z_neg: bridge,
            y_pos: bridge_top,
            y_neg: bridge_bottom,
        })
        .with_all_rotations()
        .with_weight(0.05)
        .instance();
    assets.push(asset("bridge"));

    // Small rocks and cactuses
    let sand_prop = SocketsCartesian3D::Simple {
        x_pos: sand_prop_border,
        x_neg: sand_prop_border,
        z_pos: sand_prop_border,
        z_neg: sand_prop_border,
        y_pos: sand_prop_top,
        y_neg: sand_prop_bottom,
    }
    .to_template()
    .with_all_rotations()
    .with_weight(0.25);

    models.create(sand_prop.clone());
    assets.push(vec![AssetDef::new("cactus")
        .with_grid_offset(GridDelta::new(0, -1, 0))
        .with_component(CustomComponents::ScaleRdm(ScaleRandomizer))
        .with_component(CustomComponents::RotRdm(RotationRandomizer))]);

    models.create(sand_prop.clone().with_weight(0.4));
    assets.push(vec![AssetDef::new("small_rock")
        .with_grid_offset(GridDelta::new(0, -1, 0))
        .with_component(CustomComponents::ScaleRdm(ScaleRandomizer))
        .with_component(CustomComponents::RotRdm(RotationRandomizer))]);

    const WINDMILLS_WEIGHT: f32 = 0.005;
    models
        .create(SocketsCartesian3D::Simple {
            x_pos: windmill_side,
            x_neg: windmill_side,
            z_pos: windmill_side,
            z_neg: windmill_side,
            y_pos: windmill_base_top,
            y_neg: windmill_base_bottom,
        })
        .with_all_rotations()
        .with_weight(WINDMILLS_WEIGHT);
    assets.push(asset("windmill_base"));

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: windmill_side,
            x_neg: windmill_side,
            z_pos: windmill_side,
            z_neg: windmill_side,
            y_pos: windmill_cap_top,
            y_neg: windmill_cap_bottom,
        })
        .with_weight(WINDMILLS_WEIGHT);
    assets.push(vec![
        AssetDef::new("windmill_top"),
        AssetDef::new("windmill_vane"),
        AssetDef::new("windmill_blades")
            .with_offset(Vec3::new(0., 0.7 * BLOCK_SIZE, 0.))
            .with_component(CustomComponents::Rot(WindRotation)),
    ]);

    // For this generation, our rotation axis is Y+, so we define connection on the Y axis with `add_rotated_connection` for sockets that still need to be compatible when rotated.
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
            vec![ModelRotation::Rot180, ModelRotation::Rot270],
            vec![rock_border_top, ground_rock_border_top],
        )
        // Small rocks & Cactuses
        .add_connection(sand_prop_border, vec![void, rock_border, bridge_side])
        .add_rotated_connections(vec![
            (sand_prop_bottom, vec![sand_top]),
            (sand_prop_top, vec![void_bottom, bridge_bottom]),
        ])
        // Windmills
        .add_connection(windmill_side, vec![void, rock_border, bridge_side])
        .add_rotated_connections(vec![
            (windmill_base_bottom, vec![rock_top]),
            (windmill_base_top, vec![windmill_cap_bottom]),
            (windmill_cap_top, vec![void_bottom]),
        ]);

    // We add a debug name to the models from their first asset name
    for model in models.models_mut() {
        model.with_name(
            assets[model.index()]
                .first()
                .unwrap_or(&AssetDef::new("void"))
                .path(),
        );
    }

    (
        void_instance,
        sand_instance,
        water_instance,
        bridge_instance,
        assets,
        models,
        sockets,
    )
}

#[derive(Component, Clone)]
pub struct WindRotation;

#[derive(Component, Clone)]
pub struct ScaleRandomizer;

#[derive(Component, Clone)]
pub struct RotationRandomizer;

#[derive(Clone)]
pub enum CustomComponents {
    Rot(WindRotation),
    ScaleRdm(ScaleRandomizer),
    RotRdm(RotationRandomizer),
}

impl ComponentSpawner for CustomComponents {
    fn insert(&self, command: &mut bevy::ecs::system::EntityCommands) {
        match self {
            CustomComponents::Rot(rot) => command.insert(rot.clone()),
            CustomComponents::ScaleRdm(sc) => command.insert(sc.clone()),
            CustomComponents::RotRdm(rot) => command.insert(rot.clone()),
        };
    }
}
