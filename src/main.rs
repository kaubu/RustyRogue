use tcod::colors::*;
use tcod::console::*;

// Actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

const LIMIT_FPS: i32 = 20;  // 20 frames-per-second maximum

struct Tcod {
    root: Root,
    con: Offscreen,
}

/// This is a generic object: the player, a monster, an item, the stairs, etc…
/// 
/// It's always represented by a char on screen.
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
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    /// Set the colour and then draw the character that represents this object
    /// at its position.
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.colour);
        con.put_char(self.x, self.y, self.sprite, BackgroundFlag::None);
    }
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("RustyRogue")
        .init();

    let con = Offscreen::new(SCREEN_WIDTH, SCREEN_HEIGHT);

    let mut tcod = Tcod {
        root,
        con,
    };

    tcod::system::set_fps(LIMIT_FPS);

    let mut centre_x = SCREEN_WIDTH / 2;
    let mut centre_y = SCREEN_HEIGHT / 2;

    // Create the object representing the player
    let player = Object::new(
        centre_x,
        centre_y,
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

    while !tcod.root.window_closed() {
        // Clear the screen of the previous frame
        tcod.con.clear();

        // 'Blit' the contents of "con" to the root console and present it
        blit(
            &tcod.con,
            (0, 0),
            (SCREEN_WIDTH, SCREEN_HEIGHT),
            &mut tcod.root,
            (0, 0),
            1.0,
            1.0,
        );

        tcod.root.flush();
        // commenting the below code out, as it waits for keypresses twice
        // tcod.root.wait_for_keypress(true);

        // Handle keys and exit game if needed
        let player = &mut objects[0];
        let exit = handle_keys(&mut tcod, player);
        if exit {
            break;
        }
    }
}

fn handle_keys(tcod: &mut Tcod, player: &mut Object) -> bool {
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
        Key { code: Up, .. } => player.move_by(0, -1),
        Key { code: Down, .. } => player.move_by(0, 1),
        Key { code: Left, .. } => player.move_by(-1, 0),
        Key { code: Right, .. } => player.move_by(1, 0),
        
        _ => {},
    }
    
    false
}