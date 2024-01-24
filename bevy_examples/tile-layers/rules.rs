use bevy_examples::utils::AssetDef;
use bevy_ghx_proc_gen::proc_gen::{
    generator::{
        model::{ModelCollection, ModelRotation, ModelTemplate},
        socket::{Socket, SocketCollection, SocketsCartesian3D},
    },
    grid::direction::{Cartesian3D, Direction, GridDelta},
};

const UP_AXIS: Direction = Direction::ZForward;

pub(crate) fn rules_and_assets() -> (
    Vec<Vec<AssetDef>>,
    ModelCollection<Cartesian3D>,
    SocketCollection,
) {
    let mut sockets = SocketCollection::new();

    // Create our sockets
    let mut s = || -> Socket { sockets.create() };
    let (void, dirt) = (s(), s());
    let (layer_0_down, layer_0_up) = (s(), s());

    let (grass, void_and_grass, grass_and_void) = (s(), s(), s());
    let (layer_1_down, layer_1_up, grass_up) = (s(), s(), s());

    let yellow_grass_down = s();
    let (layer_2_down, layer_2_up) = (s(), s());

    let (water, void_and_water, water_and_void) = (s(), s(), s());
    let (layer_3_down, layer_3_up, ground_up) = (s(), s(), s());

    let (layer_4_down, layer_4_up, props_down) = (s(), s(), s());
    let (big_tree_1_base, big_tree_2_base) = (s(), s());

    // Create our models. We declare our assets at the same time for clarity (index of the model matches the index of the assets to spawn).

    let mut models = ModelCollection::<Cartesian3D>::new();
    let mut assets = Vec::new();

    // Utility functions to declare assets & models
    let asset = |str| -> Vec<AssetDef> { vec![AssetDef::new(str)] };
    let mut define_model = |model_assets: Vec<AssetDef>, template: ModelTemplate<Cartesian3D>| {
        // For models, we add a debug name from to them their first asset
        models.create(template).with_name(
            model_assets
                .first()
                .unwrap_or(&AssetDef::new("void"))
                .path(),
        );
        assets.push(model_assets);
    };

    // ---------------------------- Layer 0 ----------------------------

    define_model(
        asset("dirt"),
        SocketsCartesian3D::Simple {
            x_pos: dirt,
            x_neg: dirt,
            z_pos: layer_0_up,
            z_neg: layer_0_down,
            y_pos: dirt,
            y_neg: dirt,
        }
        .to_template()
        .with_weight(20.),
    );

    // ---------------------------- Layer 1 ----------------------------

    define_model(
        vec![],
        SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: void,
            z_pos: layer_1_up,
            z_neg: layer_1_down,
            y_pos: void,
            y_neg: void,
        }
        .to_template(),
    );

    define_model(
        asset("green_grass"),
        SocketsCartesian3D::Multiple {
            x_pos: vec![grass],
            x_neg: vec![grass],
            z_pos: vec![layer_1_up, grass_up],
            z_neg: vec![layer_1_down],
            y_pos: vec![grass],
            y_neg: vec![grass],
        }
        .to_template()
        .with_weight(5.),
    );

    // Here we define models that we'll reuse multiple times
    let green_grass_corner_out = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: void,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: void,
        y_neg: grass_and_void,
    }
    .to_template();
    let green_grass_corner_in = SocketsCartesian3D::Simple {
        x_pos: grass_and_void,
        x_neg: grass,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: grass,
        y_neg: void_and_grass,
    }
    .to_template();
    let green_grass_side = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: grass_and_void,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: void,
        y_neg: grass,
    }
    .to_template();

    define_model(
        asset("green_grass_corner_out_tl"),
        green_grass_corner_out.clone(),
    );
    define_model(
        asset("green_grass_corner_out_bl"),
        green_grass_corner_out.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("green_grass_corner_out_br"),
        green_grass_corner_out.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("green_grass_corner_out_tr"),
        green_grass_corner_out.rotated(ModelRotation::Rot270, UP_AXIS),
    );
    define_model(
        asset("green_grass_corner_in_tl"),
        green_grass_corner_in.clone(),
    );
    define_model(
        asset("green_grass_corner_in_bl"),
        green_grass_corner_in.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("green_grass_corner_in_br"),
        green_grass_corner_in.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("green_grass_corner_in_tr"),
        green_grass_corner_in.rotated(ModelRotation::Rot270, UP_AXIS),
    );
    define_model(asset("green_grass_side_t"), green_grass_side.clone());
    define_model(
        asset("green_grass_side_l"),
        green_grass_side.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("green_grass_side_b"),
        green_grass_side.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("green_grass_side_r"),
        green_grass_side.rotated(ModelRotation::Rot270, UP_AXIS),
    );

    // ---------------------------- Layer 2 ----------------------------

    define_model(
        vec![], // Layer 2 Void
        SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: void,
            z_pos: layer_2_up,
            z_neg: layer_2_down,
            y_pos: void,
            y_neg: void,
        }
        .to_template(),
    );

    define_model(
        asset("yellow_grass"),
        SocketsCartesian3D::Simple {
            x_pos: grass,
            x_neg: grass,
            z_pos: layer_2_up,
            z_neg: layer_2_down,
            y_pos: grass,
            y_neg: grass,
        }
        .to_template(),
    );

    let yellow_grass_corner_out = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: void,
        z_pos: layer_2_up,
        z_neg: yellow_grass_down,
        y_pos: void,
        y_neg: grass_and_void,
    }
    .to_template();
    let yellow_grass_corner_in = SocketsCartesian3D::Simple {
        x_pos: grass_and_void,
        x_neg: grass,
        z_pos: layer_2_up,
        z_neg: yellow_grass_down,
        y_pos: grass,
        y_neg: void_and_grass,
    }
    .to_template();
    let yellow_grass_side = SocketsCartesian3D::Simple {
        x_pos: void_and_grass,
        x_neg: grass_and_void,
        z_pos: layer_2_up,
        z_neg: yellow_grass_down,
        y_pos: void,
        y_neg: grass,
    }
    .to_template();

    define_model(
        asset("yellow_grass_corner_out_tl"),
        yellow_grass_corner_out.clone(),
    );
    define_model(
        asset("yellow_grass_corner_out_bl"),
        yellow_grass_corner_out.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_corner_out_br"),
        yellow_grass_corner_out.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_corner_out_tr"),
        yellow_grass_corner_out.rotated(ModelRotation::Rot270, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_corner_in_tl"),
        yellow_grass_corner_in.clone(),
    );
    define_model(
        asset("yellow_grass_corner_in_bl"),
        yellow_grass_corner_in.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_corner_in_br"),
        yellow_grass_corner_in.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_corner_in_tr"),
        yellow_grass_corner_in.rotated(ModelRotation::Rot270, UP_AXIS),
    );
    define_model(asset("yellow_grass_side_t"), yellow_grass_side.clone());
    define_model(
        asset("yellow_grass_side_l"),
        yellow_grass_side.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_side_b"),
        yellow_grass_side.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("yellow_grass_side_r"),
        yellow_grass_side.rotated(ModelRotation::Rot270, UP_AXIS),
    );

    // ---------------------------- Layer 3 ----------------------------

    define_model(
        vec![], // Layer 3 Void
        SocketsCartesian3D::Multiple {
            x_pos: vec![void],
            x_neg: vec![void],
            z_pos: vec![layer_3_up, ground_up],
            z_neg: vec![layer_3_down],
            y_pos: vec![void],
            y_neg: vec![void],
        }
        .to_template(),
    );

    define_model(
        asset("water"),
        SocketsCartesian3D::Simple {
            x_pos: water,
            x_neg: water,
            z_pos: layer_3_up,
            z_neg: layer_3_down,
            y_pos: water,
            y_neg: water,
        }
        .to_template()
        .with_weight(10. * WATER_WEIGHT),
    );

    const WATER_WEIGHT: f32 = 0.02;
    let water_corner_out = SocketsCartesian3D::Simple {
        x_pos: void_and_water,
        x_neg: void,
        z_pos: layer_3_up,
        z_neg: layer_3_down,
        y_pos: void,
        y_neg: water_and_void,
    }
    .to_template()
    .with_weight(WATER_WEIGHT);
    let water_corner_in = SocketsCartesian3D::Simple {
        x_pos: water_and_void,
        x_neg: water,
        z_pos: layer_3_up,
        z_neg: layer_3_down,
        y_pos: water,
        y_neg: void_and_water,
    }
    .to_template()
    .with_weight(WATER_WEIGHT);
    let water_side = SocketsCartesian3D::Simple {
        x_pos: void_and_water,
        x_neg: water_and_void,
        z_pos: layer_3_up,
        z_neg: layer_3_down,
        y_pos: void,
        y_neg: water,
    }
    .to_template()
    .with_weight(WATER_WEIGHT);

    define_model(asset("water_corner_out_tl"), water_corner_out.clone());
    define_model(
        asset("water_corner_out_bl"),
        water_corner_out.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("water_corner_out_br"),
        water_corner_out.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("water_corner_out_tr"),
        water_corner_out.rotated(ModelRotation::Rot270, UP_AXIS),
    );
    define_model(asset("water_corner_in_tl"), water_corner_in.clone());
    define_model(
        asset("water_corner_in_bl"),
        water_corner_in.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("water_corner_in_br"),
        water_corner_in.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("water_corner_in_tr"),
        water_corner_in.rotated(ModelRotation::Rot270, UP_AXIS),
    );
    define_model(asset("water_side_t"), water_side.clone());
    define_model(
        asset("water_side_l"),
        water_side.rotated(ModelRotation::Rot90, UP_AXIS),
    );
    define_model(
        asset("water_side_b"),
        water_side.rotated(ModelRotation::Rot180, UP_AXIS),
    );
    define_model(
        asset("water_side_r"),
        water_side.rotated(ModelRotation::Rot270, UP_AXIS),
    );

    // ---------------------------- Layer 4 ----------------------------

    define_model(
        vec![], // Layer 4 Void
        SocketsCartesian3D::Multiple {
            x_pos: vec![void],
            x_neg: vec![void],
            z_pos: vec![layer_4_up],
            z_neg: vec![layer_4_down],
            y_pos: vec![void],
            y_neg: vec![void],
        }
        .to_template(),
    );

    const PROPS_WEIGHT: f32 = 0.025;
    const ROCKS_WEIGHT: f32 = 0.008;
    const PLANTS_WEIGHT: f32 = 0.025;
    const STUMPS_WEIGHT: f32 = 0.012;
    let prop = SocketsCartesian3D::Simple {
        x_pos: void,
        x_neg: void,
        z_pos: layer_4_up,
        z_neg: props_down,
        y_pos: void,
        y_neg: void,
    }
    .to_template()
    .with_weight(PROPS_WEIGHT);
    let plant_prop = prop.clone().with_weight(PLANTS_WEIGHT);
    let stump_prop = prop.clone().with_weight(STUMPS_WEIGHT);
    let rock_prop = prop.clone().with_weight(ROCKS_WEIGHT);

    // We define 2 assets here for 1 model. Both will be spawned when the model is selected.
    // We only need the generator to know about the tree base, but in the engine, we want
    // to spawn and see the tree leaves on top
    define_model(
        vec![
            AssetDef::new("small_tree_bottom"),
            AssetDef::new("small_tree_top").with_grid_offset(GridDelta::new(0, 1, 0)),
        ],
        plant_prop.clone(),
    );
    define_model(
        vec![
            AssetDef::new("big_tree_1_bl"),
            AssetDef::new("big_tree_1_tl").with_grid_offset(GridDelta::new(0, 1, 0)),
        ],
        SocketsCartesian3D::Simple {
            x_pos: big_tree_1_base,
            x_neg: void,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        }
        .to_template()
        .with_weight(PROPS_WEIGHT),
    );
    define_model(
        vec![
            AssetDef::new("big_tree_1_br"),
            AssetDef::new("big_tree_1_tr").with_grid_offset(GridDelta::new(0, 1, 0)),
        ],
        SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: big_tree_1_base,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        }
        .to_template()
        .with_weight(PROPS_WEIGHT),
    );
    define_model(
        vec![
            AssetDef::new("big_tree_2_bl"),
            AssetDef::new("big_tree_2_tl").with_grid_offset(GridDelta::new(0, 1, 0)),
        ],
        SocketsCartesian3D::Simple {
            x_pos: big_tree_2_base,
            x_neg: void,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        }
        .to_template()
        .with_weight(PROPS_WEIGHT),
    );
    define_model(
        vec![
            AssetDef::new("big_tree_2_br"),
            AssetDef::new("big_tree_2_tr").with_grid_offset(GridDelta::new(0, 1, 0)),
        ],
        SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: big_tree_2_base,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        }
        .to_template()
        .with_weight(PROPS_WEIGHT),
    );
    // Here we reuse the same models to create variations. (We could also have 1 model, and multiple assets, with the spawner picking one of the assets at random)
    define_model(asset("tree_stump_1"), stump_prop.clone());
    define_model(asset("tree_stump_2"), stump_prop.clone());
    define_model(asset("tree_stump_3"), stump_prop.clone());
    define_model(asset("rock_1"), rock_prop.clone());
    define_model(asset("rock_2"), rock_prop.clone());
    define_model(asset("rock_3"), rock_prop.clone());
    define_model(asset("rock_4"), rock_prop.clone());
    define_model(asset("plant_1"), plant_prop.clone());
    define_model(asset("plant_2"), plant_prop.clone());
    define_model(asset("plant_3"), plant_prop.clone());
    define_model(asset("plant_4"), plant_prop.clone());

    sockets
        .add_connections(vec![
            (dirt, vec![dirt]),
            (void, vec![void]),
            (grass, vec![grass]),
            (void_and_grass, vec![grass_and_void]),
            (water, vec![water]),
            (water_and_void, vec![void_and_water]),
            (big_tree_1_base, vec![big_tree_1_base]),
            (big_tree_2_base, vec![big_tree_2_base]),
        ])
        // For this generation, our rotation axis is Z+, so we define connection on the Z axis with `add_rotated_connection` for sockets that still need to be compatible when rotated.
        // Note: But in reality, in this example, we don't really need it. None of our models uses any rotation, apart from ModelRotation::Rot0 (notice that there's no call to `with_rotations` on any of the models).
        // Simply using `add_connections` would give the same result (it allows connections with relative_rotation = Rot0)
        .add_rotated_connections(vec![
            (layer_0_up, vec![layer_1_down]),
            (layer_1_up, vec![layer_2_down]),
            (layer_2_up, vec![layer_3_down]),
            (layer_3_up, vec![layer_4_down]),
            (yellow_grass_down, vec![grass_up]),
            (props_down, vec![ground_up]),
        ]);

    (assets, models, sockets)
}
