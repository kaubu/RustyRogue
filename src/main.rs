use std::cmp;
use tcod::colors;
use tcod::colors::*;
use tcod::console::*;
use rand::Rng;
use tcod::map::{FovAlgorithm, Map as FovMap};

// Actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

const LIMIT_FPS: i32 = 20;  // 20 frames-per-second maximum

// Size of the map
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const COLOUR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOUR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOUR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOUR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };

// Parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic; // Default FOV algorithm
const FOV_LIGHT_WALLS: bool = true; // Whether to light walls or not
const TORCH_RADIUS: i32 = 10;

const MAX_ROOM_MONSTERS: i32 = 3;

// Player will always be the first object
const PLAYER: usize = 0;

struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

/// This is a generic object: the player, a monster, an item, the stairs, etc…
/// 
/// It's always represented by a char on screen.
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    /// The character displayed on screen
    sprite: char,
    colour: Color,
    name: String,
    /// If the object blocks the character or not
    blocks: bool,
    alive: bool,
}

impl Object {
    pub fn new(
        x: i32,
        y: i32,
        sprite: char,
        color: Color,
        name: &str,
        blocks: bool
    ) -> Self {
        Object {
            x,
            y,
            sprite,
            colour: color,
            name: name.into(),
            blocks,
            alive: false,
        }
    }

    /// Set the colour and then draw the character that represents this object
    /// at its position.
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.colour);
        con.put_char(self.x, self.y, self.sprite, BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}

/// A tile of the map and its properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    /// If the tile blocks anything from going through it
    blocked: bool,
    /// If the tile has been encountered before, for Fog of War
    explored: bool,
    /// If the tile blocks the sight of things behind it
    block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            explored: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            explored: false,
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

    pub fn centre(&self) -> (i32, i32) {
        let centre_x = (self.x1 + self.x2) / 2;
        let centre_y = (self.y1 + self.y2) / 2;
        (centre_x, centre_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        // Returns true if this rectangle intersects with another one
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("RustyRogue")
        .init();

    let mut tcod = Tcod {
        root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    tcod::system::set_fps(LIMIT_FPS);

    let centre_x = SCREEN_WIDTH / 2;
    let centre_y = SCREEN_HEIGHT / 2;

    // Create the object representing the player
    let mut player = Object::new(
        0,
        0,
        '@',
        WHITE,
        "Player",
        true,
    );
    player.alive = true;

    // Create an NPC
    let mut npc = Object::new(
        centre_x - 5,
        centre_y,
        '@',
        YELLOW,
        "NPC",
        true,
    );
    npc.alive = true;

    // The list of objects with those two
    let mut objects = vec![player, npc];

    let mut game = Game {
        // Generate map (at this point it's not drawn on the screen)
        map: make_map(&mut objects),
    };

    // Populate the FOV map, according to the generated map
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            );
        }
    }

    // Force FOV "recompute" first time through game loop
    let mut previous_player_position = (-1, -1);

    // The main game loop
    while !tcod.root.window_closed() {
        // Clear the screen of the previous frame
        tcod.con.clear();

        // Render the screen
        let fov_recompute = previous_player_position != (
            objects[PLAYER].x,
            objects[PLAYER].y
        );
        render_all(&mut tcod, &mut game, &objects, fov_recompute);

        tcod.root.flush();
        // Commenting the below code out, as it waits for keypresses twice
        // tcod.root.wait_for_keypress(true);

        // Handle keys and exit game if needed
        // let player = &mut objects[PLAYER];
        // previous_player_position = (player.x, player.y);
        // let exit = handle_keys(&mut tcod, &game, player);
        // if exit {
        //     break;
        // }
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(
            &mut tcod,
            &game,
            &mut objects
        );
        if player_action == PlayerAction::Exit {
            break;
        }

        // Let monsters take their turn
        if objects[PLAYER].alive
            && player_action != PlayerAction::DidntTakeTurn {
            for object in &objects {
                // Only if object is not player
                if (object as *const _) != (&objects[PLAYER] as *const _) {
                    println!("The {} growls!", object.name);
                }
            }
        }
    }
}

fn handle_keys(
    tcod: &mut Tcod,
    game: &Game,
    objects: &mut Vec<Object>,
) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    return match (key, key.text(), player_alive) {
        // Alt+Enter: Toggle Fullscreen
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
            _
        ) => {
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        // Exit game
        (Key { code: Escape, .. }, _, _) => Exit,

        // Movement keys
        (Key { code: Up, .. }, _, _) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        },
        (Key { code: Down, .. }, _, _) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        },
        (Key { code: Left, .. }, _, _) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        },
        (Key { code: Right, .. }, _, _) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        },
        
        _ => DidntTakeTurn,
    }
}

fn make_map(objects: &mut Vec<Object>) -> Map {
    // Fill map with "blocked" tiles
    let mut map = vec![
        vec![Tile::wall(); MAP_HEIGHT as usize];
        MAP_WIDTH as usize
    ];

    let mut rooms = Vec::new();

    for _ in 0..MAX_ROOMS {
        // Random width and height
        let w = rand::thread_rng().gen_range(
            ROOM_MIN_SIZE,
            ROOM_MAX_SIZE + 1
        );
        let h = rand::thread_rng().gen_range(
            ROOM_MIN_SIZE,
            ROOM_MAX_SIZE + 1
        );
        // Random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // Run through the other rooms and see if they intersect with this one
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));
        
        if !failed {
            // This means that there are no intersections, so this room is
            // valid

            // "Paint" it to the map's tiles
            create_room(new_room, &mut map);

            // Add some content to this room, such as monsters
            place_objects(new_room, objects, &map);

            // Centre coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.centre();

            if rooms.is_empty() {
                // This is the first room, where the player starts at
                objects[PLAYER].set_pos(new_x, new_y);
            } else {
                // All rooms after the first:
                // Connect it to the previous room with a tunnel

                // Centre coordinates of the previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1]
                    .centre();

                // Toss a coin (random bool value – either true or false)
                if rand::random::<bool>() {
                    // First move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // First move vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }

            rooms.push(new_room);
        }
    }

    map
}

fn render_all(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    fov_recompute: bool,
) {
    if fov_recompute {
        // Recompute FOV if needed (the player moved or an object updated)
        let player = &objects[PLAYER];
        tcod.fov.compute_fov(
            player.x,
            player.y,
            TORCH_RADIUS,
            FOV_LIGHT_WALLS,
            FOV_ALGO
        );
    }

    // Draw all objects in the list
    for object in objects {
        if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        }
    }

    // Go through all tiles, and set their background colour
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let colour = match (visible, wall) {
                // Outside FOV:
                (false, true) => COLOUR_DARK_WALL,
                (false, false) => COLOUR_DARK_GROUND,
                
                // Inside FOV:
                (true, true) => COLOUR_LIGHT_WALL,
                (true, false) => COLOUR_LIGHT_GROUND,
            };

            let explored = &mut game
                .map[x as usize][y as usize]
                .explored;
            if visible {
                // Since it's visible, explore it
                *explored = true;
            }
            if *explored {
                // Show explored tiles only (any visible tile is explored
                // already)
                tcod.con.set_char_background(
                    x,
                    y,
                    colour,
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

fn place_objects(room: Rect, objects: &mut Vec<Object>, map: &Map) {
    // Choose random number of monsters
    let num_monsters = rand::thread_rng()
        .gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // Choose random spot for the monster
        let x = rand::thread_rng()
            .gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng()
            .gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            // 80% chance of getting an Orc
            let mut monster = if rand::random::<f32>() < 0.8 {
                // Create an Orc
                Object::new(
                    x,
                    y,
                    'O',
                    colors::DESATURATED_GREEN,
                    "Orc",
                    true
                )
            } else {
                // Create a Troll
                Object::new(
                    x,
                    y,
                    'T',
                    colors::DARKER_GREEN,
                    "Troll",
                    true
                )
            };

            monster.alive = true;
            objects.push(monster);
        }
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // First, test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // Now, check for any blocking objects
    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}

/// Move by the given amount
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn player_move_or_attack(
    dx: i32,
    dy: i32,
    game: &Game,
    objects: &mut [Object]
) {
    // The coordinates the player is moving to/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // Try to find an attackable target there
    let target_id = objects.iter().position(|object| {
        object.pos() == (x, y)
    });

    // Attack if target is found, move otherwise
    match target_id {
        Some(target_id) => {
            println!(
                "The {} laughs at your puny efforts to attack them!",
                objects[target_id].name
            );
        },
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}
