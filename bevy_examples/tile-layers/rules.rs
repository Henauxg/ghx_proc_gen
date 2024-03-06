use bevy_examples::utils::AssetDef;
use bevy_ghx_grid::ghx_grid::{
    coordinate_system::Cartesian3D,
    direction::{Direction, GridDelta},
};
use bevy_ghx_proc_gen::proc_gen::generator::{
    model::{ModelCollection, ModelRotation},
    socket::{Socket, SocketCollection, SocketsCartesian3D},
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

    // ---------------------------- Layer 0 ----------------------------

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: dirt,
            x_neg: dirt,
            z_pos: layer_0_up,
            z_neg: layer_0_down,
            y_pos: dirt,
            y_neg: dirt,
        })
        .with_weight(20.);
    assets.push(asset("dirt"));

    // ---------------------------- Layer 1 ----------------------------

    models.create(SocketsCartesian3D::Simple {
        x_pos: void,
        x_neg: void,
        z_pos: layer_1_up,
        z_neg: layer_1_down,
        y_pos: void,
        y_neg: void,
    });
    assets.push(vec![]);

    models
        .create(SocketsCartesian3D::Multiple {
            x_pos: vec![grass],
            x_neg: vec![grass],
            z_pos: vec![layer_1_up, grass_up],
            z_neg: vec![layer_1_down],
            y_pos: vec![grass],
            y_neg: vec![grass],
        })
        .with_weight(5.);
    assets.push(asset("green_grass"));

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

    models.create(green_grass_corner_out.clone());
    assets.push(asset("green_grass_corner_out_tl"));

    models.create(green_grass_corner_out.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("green_grass_corner_out_bl"));

    models.create(green_grass_corner_out.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("green_grass_corner_out_br"));

    models.create(green_grass_corner_out.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("green_grass_corner_out_tr"));

    models.create(green_grass_corner_in.clone());
    assets.push(asset("green_grass_corner_in_tl"));

    models.create(green_grass_corner_in.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("green_grass_corner_in_bl"));

    models.create(green_grass_corner_in.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("green_grass_corner_in_br"));

    models.create(green_grass_corner_in.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("green_grass_corner_in_tr"));

    models.create(green_grass_side.clone());
    assets.push(asset("green_grass_side_t"));

    models.create(green_grass_side.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("green_grass_side_l"));

    models.create(green_grass_side.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("green_grass_side_b"));

    models.create(green_grass_side.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("green_grass_side_r"));

    // ---------------------------- Layer 2 ----------------------------

    models.create(SocketsCartesian3D::Simple {
        x_pos: void,
        x_neg: void,
        z_pos: layer_2_up,
        z_neg: layer_2_down,
        y_pos: void,
        y_neg: void,
    });
    assets.push(vec![]); // Layer 2 Void

    models.create(SocketsCartesian3D::Simple {
        x_pos: grass,
        x_neg: grass,
        z_pos: layer_2_up,
        z_neg: layer_2_down,
        y_pos: grass,
        y_neg: grass,
    });
    assets.push(asset("yellow_grass"));

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

    models.create(yellow_grass_corner_out.clone());
    assets.push(asset("yellow_grass_corner_out_tl"));

    models.create(yellow_grass_corner_out.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("yellow_grass_corner_out_bl"));

    models.create(yellow_grass_corner_out.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("yellow_grass_corner_out_br"));

    models.create(yellow_grass_corner_out.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("yellow_grass_corner_out_tr"));

    models.create(yellow_grass_corner_in.clone());
    assets.push(asset("yellow_grass_corner_in_tl"));

    models.create(yellow_grass_corner_in.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("yellow_grass_corner_in_bl"));

    models.create(yellow_grass_corner_in.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("yellow_grass_corner_in_br"));

    models.create(yellow_grass_corner_in.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("yellow_grass_corner_in_tr"));

    models.create(yellow_grass_side.clone());
    assets.push(asset("yellow_grass_side_t"));

    models.create(yellow_grass_side.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("yellow_grass_side_l"));

    models.create(yellow_grass_side.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("yellow_grass_side_b"));

    models.create(yellow_grass_side.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("yellow_grass_side_r"));

    // ---------------------------- Layer 3 ----------------------------

    models.create(SocketsCartesian3D::Multiple {
        x_pos: vec![void],
        x_neg: vec![void],
        z_pos: vec![layer_3_up, ground_up],
        z_neg: vec![layer_3_down],
        y_pos: vec![void],
        y_neg: vec![void],
    });
    assets.push(vec![]); // Layer 3 Void

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: water,
            x_neg: water,
            z_pos: layer_3_up,
            z_neg: layer_3_down,
            y_pos: water,
            y_neg: water,
        })
        .with_weight(10. * WATER_WEIGHT);
    assets.push(asset("water"));

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

    models.create(water_corner_out.clone());
    assets.push(asset("water_corner_out_tl"));

    models.create(water_corner_out.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("water_corner_out_bl"));

    models.create(water_corner_out.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("water_corner_out_br"));

    models.create(water_corner_out.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("water_corner_out_tr"));

    models.create(water_corner_in.clone());
    assets.push(asset("water_corner_in_tl"));

    models.create(water_corner_in.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("water_corner_in_bl"));

    models.create(water_corner_in.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("water_corner_in_br"));

    models.create(water_corner_in.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("water_corner_in_tr"));

    models.create(water_side.clone());
    assets.push(asset("water_side_t"));

    models.create(water_side.rotated(ModelRotation::Rot90, UP_AXIS));
    assets.push(asset("water_side_l"));

    models.create(water_side.rotated(ModelRotation::Rot180, UP_AXIS));
    assets.push(asset("water_side_b"));

    models.create(water_side.rotated(ModelRotation::Rot270, UP_AXIS));
    assets.push(asset("water_side_r"));

    // ---------------------------- Layer 4 ----------------------------

    models.create(SocketsCartesian3D::Multiple {
        x_pos: vec![void],
        x_neg: vec![void],
        z_pos: vec![layer_4_up],
        z_neg: vec![layer_4_down],
        y_pos: vec![void],
        y_neg: vec![void],
    });
    assets.push(vec![]); // Layer 4 Void

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
    // (We could also just have 1 asset with a 32x64 size)
    models.create(plant_prop.clone());
    assets.push(vec![
        AssetDef::new("small_tree_bottom"),
        AssetDef::new("small_tree_top").with_grid_offset(GridDelta::new(0, 1, 0)),
    ]);

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: big_tree_1_base,
            x_neg: void,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        })
        .with_weight(PROPS_WEIGHT);
    assets.push(vec![
        AssetDef::new("big_tree_1_bl"),
        AssetDef::new("big_tree_1_tl").with_grid_offset(GridDelta::new(0, 1, 0)),
    ]);

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: big_tree_1_base,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        })
        .with_weight(PROPS_WEIGHT);
    assets.push(vec![
        AssetDef::new("big_tree_1_br"),
        AssetDef::new("big_tree_1_tr").with_grid_offset(GridDelta::new(0, 1, 0)),
    ]);

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: big_tree_2_base,
            x_neg: void,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        })
        .with_weight(PROPS_WEIGHT);
    assets.push(vec![
        AssetDef::new("big_tree_2_bl"),
        AssetDef::new("big_tree_2_tl").with_grid_offset(GridDelta::new(0, 1, 0)),
    ]);

    models
        .create(SocketsCartesian3D::Simple {
            x_pos: void,
            x_neg: big_tree_2_base,
            z_pos: layer_4_up,
            z_neg: props_down,
            y_pos: void,
            y_neg: void,
        })
        .with_weight(PROPS_WEIGHT);
    assets.push(vec![
        AssetDef::new("big_tree_2_br"),
        AssetDef::new("big_tree_2_tr").with_grid_offset(GridDelta::new(0, 1, 0)),
    ]);

    // Here we reuse the same models to create variations. (We could also have 1 model, and multiple assets, with the spawner picking one of the assets at random)
    models.create(stump_prop.clone());
    assets.push(asset("tree_stump_1"));

    models.create(stump_prop.clone());
    assets.push(asset("tree_stump_2"));

    models.create(stump_prop.clone());
    assets.push(asset("tree_stump_3"));

    models.create(rock_prop.clone());
    assets.push(asset("rock_1"));

    models.create(rock_prop.clone());
    assets.push(asset("rock_2"));

    models.create(rock_prop.clone());
    assets.push(asset("rock_3"));

    models.create(rock_prop.clone());
    assets.push(asset("rock_4"));

    models.create(plant_prop.clone());
    assets.push(asset("plant_1"));

    models.create(plant_prop.clone());
    assets.push(asset("plant_2"));

    models.create(plant_prop.clone());
    assets.push(asset("plant_3"));

    models.create(plant_prop.clone());
    assets.push(asset("plant_4"));

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

    // We add a debug name to the models from their first asset name
    for model in models.models_mut() {
        model.with_name(
            assets[model.index()]
                .first()
                .unwrap_or(&AssetDef::new("void"))
                .path(),
        );
    }

    (assets, models, sockets)
}
