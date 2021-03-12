extern crate multiarray;
extern crate rand;
extern crate rand_pcg;

mod cell_grid;
mod color_preset;
mod coord;
mod fontdata;
mod game;
mod guard;
mod random_map;
mod speech_bubbles;

// Global game state (not thread-safe, but this program is single-threaded)

static mut GAME: Option<game::Game> = None;

// Javascript-facing interface

#[no_mangle]
pub fn rs_start(seed0: u32, seed1: u32) -> () {
    let seed = ((seed0 as u64) << 32) + (seed1 as u64);
    let game = game::new_game(seed);
    unsafe { GAME = Some(game); }
}

#[no_mangle]
pub fn rs_on_draw(screen_size_x: i32, screen_size_y: i32) {
    if let Some(game) = unsafe { &GAME } {
        game::on_draw(&game, screen_size_x, screen_size_y);
    }
}

#[no_mangle]
pub fn rs_on_key_down(key: i32, ctrl_key_down: i32, shift_key_down: i32) -> () {
    if let Some(game) = unsafe { &mut GAME } {
        game::on_key_down(game, key, ctrl_key_down != 0, shift_key_down != 0);
    }
}

// Rust-facing interface

mod engine {
    #![allow(dead_code)]

    /// Key codes passed to game::on_key_down()

    pub const KEY_BACKSPACE: i32 = 8;
    pub const KEY_TAB: i32 = 9;
    pub const KEY_ENTER: i32 = 13;

    pub const KEY_ESCAPE: i32 = 27;
    pub const KEY_SPACE: i32 = 32;

    pub const KEY_PAGE_UP: i32 = 33;
    pub const KEY_PAGE_DOWN: i32 = 34;
    pub const KEY_END: i32 = 35;
    pub const KEY_HOME: i32 = 36;
    pub const KEY_LEFT: i32 = 37;
    pub const KEY_UP: i32 = 38;
    pub const KEY_RIGHT: i32 = 39;
    pub const KEY_DOWN: i32 = 40;
    pub const KEY_INSERT: i32 = 45;
    pub const KEY_DELETE: i32 = 46;

    pub const KEY_0: i32 = 48;
    pub const KEY_1: i32 = 49;
    pub const KEY_2: i32 = 50;
    pub const KEY_3: i32 = 51;
    pub const KEY_4: i32 = 52;
    pub const KEY_5: i32 = 53;
    pub const KEY_6: i32 = 54;
    pub const KEY_7: i32 = 55;
    pub const KEY_8: i32 = 56;
    pub const KEY_9: i32 = 57;

    pub const KEY_A: i32 = 65;
    pub const KEY_B: i32 = 66;
    pub const KEY_C: i32 = 67;
    pub const KEY_D: i32 = 68;
    pub const KEY_E: i32 = 69;
    pub const KEY_F: i32 = 70;
    pub const KEY_G: i32 = 71;
    pub const KEY_H: i32 = 72;
    pub const KEY_I: i32 = 73;
    pub const KEY_J: i32 = 74;
    pub const KEY_K: i32 = 75;
    pub const KEY_L: i32 = 76;
    pub const KEY_M: i32 = 77;
    pub const KEY_N: i32 = 78;
    pub const KEY_O: i32 = 79;
    pub const KEY_P: i32 = 80;
    pub const KEY_Q: i32 = 81;
    pub const KEY_R: i32 = 82;
    pub const KEY_S: i32 = 83;
    pub const KEY_T: i32 = 84;
    pub const KEY_U: i32 = 85;
    pub const KEY_V: i32 = 86;
    pub const KEY_W: i32 = 87;
    pub const KEY_X: i32 = 88;
    pub const KEY_Y: i32 = 89;
    pub const KEY_Z: i32 = 90;

    pub const KEY_NUMPAD0: i32 = 96;
    pub const KEY_NUMPAD1: i32 = 97;
    pub const KEY_NUMPAD2: i32 = 98;
    pub const KEY_NUMPAD3: i32 = 99;
    pub const KEY_NUMPAD4: i32 = 100;
    pub const KEY_NUMPAD5: i32 = 101;
    pub const KEY_NUMPAD6: i32 = 102;
    pub const KEY_NUMPAD7: i32 = 103;
    pub const KEY_NUMPAD8: i32 = 104;
    pub const KEY_NUMPAD9: i32 = 105;

    pub const KEY_NUMPAD_MULTIPLY: i32 = 106;
    pub const KEY_NUMPAD_ADD: i32 = 107;
    pub const KEY_NUMPAD_ENTER: i32 = 108;
    pub const KEY_NUMPAD_SUBTRACT: i32 = 109;
    pub const KEY_NUMPAD_DECIMAL: i32 = 110;
    pub const KEY_NUMPAD_DIVIDE: i32 = 111;
    pub const KEY_SEMICOLON: i32 = 186;
    pub const KEY_EQUAL: i32 = 187;
    pub const KEY_COMMA: i32 = 188;
    pub const KEY_MINUS: i32 = 189;
    pub const KEY_PERIOD: i32 = 190;
    pub const KEY_SLASH: i32 = 191;
    pub const KEY_BACKQUOTE: i32 = 192;
    pub const KEY_BRACKET_LEFT: i32 = 219;
    pub const KEY_BACKSLASH: i32 = 220;
    pub const KEY_BRACKET_RIGHT: i32 = 221;
    pub const KEY_QUOTE: i32 = 222;

    // TODO: Create an object that contains draw_rect() and draw_tile(); pass it to on_draw()
    // to ensure that draw_rect()/draw_tile() only get called during on_draw().

    /// Fill a rectangle with a solid color. Only call during game::on_draw().
    pub fn draw_rect(dest_x: i32, dest_y: i32, size_x: i32, size_y: i32, color: u32) {
        extern { fn js_draw_rect(dest_x: i32, dest_y: i32, size_x: i32, size_y: i32, color: u32); }
        unsafe { js_draw_rect(dest_x, dest_y, size_x, size_y, color) };
    }

    /// Copy a rectangular area from a texture to the screen, multiplied by a color. Only call during game::on_draw().
    pub fn draw_tile(dest_x: i32, dest_y: i32, size_x: i32, size_y: i32, color: u32, texture_index: u32, src_x: i32, src_y: i32) {
        extern { fn js_draw_tile(dest_x: i32, dest_y: i32, size_x: i32, size_y: i32, color: u32, texture_index: u32, src_x: i32, src_y: i32); }
        unsafe { js_draw_tile(dest_x, dest_y, size_x, size_y, color, texture_index, src_x, src_y); }
    }

    /// Request game::on_draw() to be called
    pub fn invalidate_screen() {
        extern { fn js_invalidate_screen(); }
        unsafe { js_invalidate_screen(); }
    }
}
