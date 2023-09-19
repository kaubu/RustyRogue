use std::cmp;
use tcod::colors::*;
use tcod::console::*;

// Actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

const LIMIT_FPS: i32 = 20;  // 20 frames-per-second maximum

// Size of the map
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const COLOUR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOUR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };

struct Tcod {
    root: Root,
    con: Offscreen,
}

/// This is a generic object: the player, a monster, an item, the stairs, etc…
/// 
/// It's always represented by a char on screen.
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    sprite: char,
    colour: Color,
}

impl Object {
    pub fn new(x: i32, y: i32, sprite: char, color: Color) -> Self {
        Object { x, y, sprite, colour: color }
    }

    /// Move by the given amount
    pub fn move_by(&mut self, dx: i32, dy: i32, game: &Game) {
        if !game.map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    /// Set the colour and then draw the character that represents this object
    /// at its position.
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.colour);
        con.put_char(self.x, self.y, self.sprite, BackgroundFlag::None);
    }
}

/// A tile of the map and its properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    /// If the tile blocks anything from going through it
    blocked: bool,
    /// If the tile blocks the sight of things behind it
    block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
        }
    }
}

type Map = Vec<Vec<Tile>>;

struct Game {
    map: Map,
}

/// A rectangle on the map, used to characterize a room.
#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("RustyRogue")
        .init();

    let con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

    let mut tcod = Tcod {
        root,
        con,
    };

    tcod::system::set_fps(LIMIT_FPS);

    let centre_x = SCREEN_WIDTH / 2;
    let centre_y = SCREEN_HEIGHT / 2;

    // Create the object representing the player
    let player = Object::new(
        25,
        23,
        '@',
        WHITE
    );

    // Create an NPC
    let npc = Object::new(
        centre_x - 5,
        centre_y,
        '@',
        YELLOW
    );

    // The list of objects with those two
    let mut objects = [player, npc];

    let game = Game {
        // Generate map (at this point it's not drawn on the screen)
        map: make_map(),
    };

    // The main game loop
    while !tcod.root.window_closed() {
        // Clear the screen of the previous frame
        tcod.con.clear();

        // Render the screen
        render_all(&mut tcod, &game, &objects);

        tcod.root.flush();
        // Commenting the below code out, as it waits for keypresses twice
        // tcod.root.wait_for_keypress(true);

        // Handle keys and exit game if needed
        let player = &mut objects[0];
        let exit = handle_keys(&mut tcod, &game, player);
        if exit {
            break;
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &Game, player: &mut Object) -> bool {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);
    match key {
        // Alt+Enter: Toggle Fullscreen
        Key {
            code: Enter,
            alt: true,
            ..
        } => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
        },
        // Exit game
        Key { code: Escape, .. } => return true,

        // Movement keys
        Key { code: Up, .. } => player.move_by(0, -1, game),
        Key { code: Down, .. } => player.move_by(0, 1, game),
        Key { code: Left, .. } => player.move_by(-1, 0, game),
        Key { code: Right, .. } => player.move_by(1, 0, game),
        
        _ => {},
    }
    
    false
}

fn make_map() -> Map {
    // Fill map with "blocked" tiles
    let mut map = vec![
        vec![Tile::wall(); MAP_HEIGHT as usize];
        MAP_WIDTH as usize
    ];

    // Create two rooms
    let room1 = Rect::new(20, 15, 10, 15);
    let room2 = Rect::new(50, 15, 10, 15);
    create_room(room1, &mut map);
    create_room(room2, &mut map);
    create_h_tunnel(25, 55, 23, &mut map);

    map
}

fn render_all(tcod: &mut Tcod, game: &Game, objects: &[Object]) {
    // Draw all objects in the list
    for object in objects {
        object.draw(&mut tcod.con);
    }

    // Go through all tiles, and set their background colour
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let wall = game.map[x as usize][y as usize].block_sight;
            if wall {
                tcod.con.set_char_background(
                    x,
                    y,
                    COLOUR_DARK_WALL,
                    BackgroundFlag::Set
                );
            } else {
                tcod.con.set_char_background(
                    x,
                    y,
                    COLOUR_DARK_GROUND,
                    BackgroundFlag::Set
                );
            }
        }
    }

    // 'Blit' the contents of "con" to the root console and present it
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

fn create_room(room: Rect, map: &mut Map) {
    // Go through the tiles in the rectangle and make them passable
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    // Horizontal tunnel. `min()` and `max()` are used in the case of `x1 > x2`
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    // Vertical tunnel
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}
