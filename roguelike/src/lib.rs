extern crate rand;
extern crate rand_pcg;

mod fontdata;
mod game;

// Global game state (not thread-safe, but this program is single-threaded)

static mut STATE: Option<game::State> = None;

// Javascript-facing interface

#[no_mangle]
pub fn rs_start(seed0: u32, seed1: u32) -> () {
	let seed = ((seed0 as u64) << 32) + (seed1 as u64);
	let state = game::new_state(seed);
	unsafe { STATE = Some(state); }
}

#[no_mangle]
pub fn rs_on_draw(screen_size_x: i32, screen_size_y: i32) {
	if let Some(state) = unsafe { &STATE } {
		game::on_draw(&state, screen_size_x, screen_size_y);
	}
}

#[no_mangle]
pub fn rs_on_key_down(key: i32, ctrl_key_down: i32, shift_key_down: i32) -> () {
	if let Some(state) = unsafe { &mut STATE } {
		game::on_key_down(state, key, ctrl_key_down != 0, shift_key_down != 0);
	}
}

// Rust-facing interface

mod engine {
	/// Key codes passed to game::on_key_down()
	pub const KEY_LEFT: i32 = 37;
	pub const KEY_UP: i32 = 38;
	pub const KEY_RIGHT: i32 = 39;
	pub const KEY_DOWN: i32 = 40;
	pub const KEY_B: i32 = 66;
	pub const KEY_H: i32 = 72;
	pub const KEY_J: i32 = 74;
	pub const KEY_K: i32 = 75;
	pub const KEY_L: i32 = 76;
	pub const KEY_N: i32 = 78;
	pub const KEY_U: i32 = 85;
	pub const KEY_Y: i32 = 89;
	pub const KEY_NUMPAD1: i32 = 97;
	pub const KEY_NUMPAD2: i32 = 98;
	pub const KEY_NUMPAD3: i32 = 99;
	pub const KEY_NUMPAD4: i32 = 100;
	pub const KEY_NUMPAD5: i32 = 101;
	pub const KEY_NUMPAD6: i32 = 102;
	pub const KEY_NUMPAD7: i32 = 103;
	pub const KEY_NUMPAD8: i32 = 104;
	pub const KEY_NUMPAD9: i32 = 105;
	pub const KEY_DECIMAL: i32 = 110;

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
