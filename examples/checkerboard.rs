use ghx_proc_gen::generator::{
    rules::RulesBuilder,
    socket::{SocketCollection, SocketsCartesian2D},
};

use {ghx_proc_gen::generator::builder::GeneratorBuilder, ghx_proc_gen::grid::GridDefinition};

fn main() {
    // A SocketCollection is what we use to create sockets and define their connections
    let mut sockets = SocketCollection::new();
    let (white, black) = (sockets.create(), sockets.create());

    // With the following, a white socket can connect to a black socket and vice-versa
    sockets.add_connection(white, vec![black]);

    // We define 2 very simple models, a white tile model with the `white` socket on each side and a black tile model with the `black` socket on each side
    let models = vec![
        SocketsCartesian2D::Mono(white).new_model(),
        SocketsCartesian2D::Mono(black).new_model(),
    ];

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
        .build()
        .unwrap();

    // Here we directly generate the whole grid, and ask for the result to be returned.
    // The generation could also be done iteratively via `generator.select_and_propagate()`, or the results could be obtained through an `Observer`
    let checker_pattern = generator.generate_and_collect_all().unwrap();

    let icons = vec!["◻️ ", "⬛"];
    for y in 0..checker_pattern.grid().size_y() {
        for x in 0..checker_pattern.grid().size_x() {
            print!("{}", icons[checker_pattern.get_2d(x, y).model_index]);
        }
        println!();
    }
}
