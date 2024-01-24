use ghx_proc_gen::{
    generator::{
        model::ModelCollection,
        rules::RulesBuilder,
        socket::{SocketCollection, SocketsCartesian2D},
    },
    grid::{direction::Cartesian2D, GridPosition},
};

use {ghx_proc_gen::generator::builder::GeneratorBuilder, ghx_proc_gen::grid::GridDefinition};

fn main() {
    // A SocketCollection is what we use to create sockets and define their connections
    let mut sockets = SocketCollection::new();
    let (white, black) = (sockets.create(), sockets.create());

    // With the following, a white socket can connect to a black socket and vice-versa
    sockets.add_connection(white, vec![black]);

    // We define 2 very simple models, a white tile model with the `white` socket on each side and a black tile model with the `black` socket on each side
    let mut models = ModelCollection::<Cartesian2D>::new();
    models.create(SocketsCartesian2D::Mono(white));
    let black_model = models.create(SocketsCartesian2D::Mono(black)).clone();

    // We give those to a RulesBuilder and get our Rules
    let rules = RulesBuilder::new_cartesian_2d(models, sockets)
        .build()
        .unwrap();

    // Like a chess board, let's do an 8x8 2d grid
    let grid = GridDefinition::new_cartesian_2d(8, 8, false, false);

    // There many more parameters you can tweak on a Generator before building it, explore the API.
    let mut generator = GeneratorBuilder::new()
        .with_rules(rules)
        .with_grid(grid)
        // Let's ensure that we make a chessboard, with a black square bottom-left
        .with_initial_nodes(vec![(GridPosition::new_xy(0, 0), black_model)])
        .build()
        .unwrap();

    // Here we directly generate the whole grid, and ask for the result to be returned.
    // The generation could also be done iteratively via `generator.select_and_propagate()`, or the results could be obtained through an `Observer`
    let (_gen_info, checker_pattern) = generator.generate_grid().unwrap();

    let icons = vec!["◻️ ", "⬛"];
    // We draw from top to bottom
    for y in (0..checker_pattern.grid().size_y()).rev() {
        for x in 0..checker_pattern.grid().size_x() {
            print!("{}", icons[checker_pattern.get_2d(x, y).model_index]);
        }
        println!();
    }
}
