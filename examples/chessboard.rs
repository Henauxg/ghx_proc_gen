use std::error::Error;

use ghx_proc_gen::{
    generator::{
        model::ModelCollection,
        rules::RulesBuilder,
        socket::{SocketCollection, SocketsCartesian2D},
    },
    ghx_grid::cartesian::{coordinates::Cartesian2D, grid::CartesianGrid},
};

use ghx_proc_gen::generator::builder::GeneratorBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // A SocketCollection is what we use to create sockets and define their connections
    let mut sockets = SocketCollection::new();
    // For this example, we will only need two sockets
    let (white, black) = (sockets.create(), sockets.create());

    // With the following, a white socket can connect to a black socket and vice-versa
    sockets.add_connection(white, vec![black]);

    let mut models = ModelCollection::<Cartesian2D>::new();
    // We define 2 very simple models: a white tile model with the `white` socket on each side
    // and a black tile model with the `black` socket on each side
    models.create(SocketsCartesian2D::Mono(white));
    // We keep the black model for later
    let black_model = models.create(SocketsCartesian2D::Mono(black)).clone();

    // We give the models and socket collection to a RulesBuilder and get our Rules
    let rules = RulesBuilder::new_cartesian_2d(models, sockets)
        .build()
        .unwrap();

    // Like a chess board, let's do an 8x8 2d grid
    let grid = CartesianGrid::new_cartesian_2d(8, 8, false, false);
    let initial_nodes = vec![(grid.index_from_coords(0, 0, 0), black_model)];

    // There many more parameters you can tweak on a Generator before building it, explore the API.
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        // Let's ensure that we make a chessboard, with a black square bottom-left
        //.with_initial_nodes(vec![(CartesianPosition::new_xy(0, 0), black_model)])?
        .with_initial_nodes(initial_nodes)?
        .build()?;

    // Here we directly generate the whole grid, and ask for the result to be returned.
    // The generation could also be done iteratively via `generator.select_and_propagate()`, or the results could be obtained through an `Observer`
    let (_gen_info, chess_pattern) = generator.generate_grid().unwrap();

    let icons = vec!["◻️ ", "⬛"];
    // We draw from top to bottom
    for y in (0..chess_pattern.grid().size_y()).rev() {
        for x in 0..chess_pattern.grid().size_x() {
            print!("{}", icons[chess_pattern.get_2d(x, y).model_index]);
        }
        println!();
    }

    Ok(())
}
