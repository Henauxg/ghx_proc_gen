use lazy_static::lazy_static;

#[derive(Clone, Copy)]
pub enum Direction {
    XForward,
    XBackward,
    YForward,
    YBackward,
    ZForward,
    ZBackward,
}

pub struct ConcreteDirection {
    dir: Direction,
    dx: i8,
    dy: i8,
    dz: i8,
}
impl ConcreteDirection {
    fn new(dir: Direction, dx: i8, dy: i8, dz: i8) -> Self {
        Self { dir, dx, dy, dz }
    }
}
impl Direction {
    pub fn index(&self) -> usize {
        match *self {
            Direction::XForward => 0,
            Direction::XBackward => 1,
            Direction::YForward => 2,
            Direction::YBackward => 3,
            Direction::ZForward => 4,
            Direction::ZBackward => 5,
        }
    }
}

pub struct DirectionSet {
    // directions: &'a [Direction],
    directions: Vec<ConcreteDirection>,
}

impl DirectionSet {}

lazy_static! {
    pub static ref CARTESIAN_2D: DirectionSet = DirectionSet {
        directions: vec![
            ConcreteDirection::new(Direction::XForward, 1, 0, 0),
            ConcreteDirection::new(Direction::XBackward, -1, 0, 0),
            ConcreteDirection::new(Direction::YForward, 0, 1, 0),
            ConcreteDirection::new(Direction::YBackward, 0, -1, 0),
        ],
    };
    pub static ref CARTESIAN_3D: DirectionSet = DirectionSet {
        directions: vec![
            ConcreteDirection::new(Direction::XForward, 1, 0, 0),
            ConcreteDirection::new(Direction::XBackward, -1, 0, 0),
            ConcreteDirection::new(Direction::YForward, 0, 1, 0),
            ConcreteDirection::new(Direction::YBackward, 0, -1, 0),
            ConcreteDirection::new(Direction::ZForward, 0, 0, 1),
            ConcreteDirection::new(Direction::ZBackward, 0, 0, -1),
        ],
    };
}

struct Grid {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    looping_x: bool,
    looping_y: bool,
    looping_z: bool,
    directions: DirectionSet,
}

impl Grid {}
