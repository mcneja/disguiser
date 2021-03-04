extern crate rand;
extern crate rand_pcg;

use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::iter::FromIterator;
use std::cmp::min;
use std::cmp::max;

type Random = rand_pcg::Pcg32;

const KEY_LEFT: i32 = 37;
const KEY_UP: i32 = 38;
const KEY_RIGHT: i32 = 39;
const KEY_DOWN: i32 = 40;
const KEY_B: i32 = 66;
const KEY_H: i32 = 72;
const KEY_J: i32 = 74;
const KEY_K: i32 = 75;
const KEY_L: i32 = 76;
const KEY_N: i32 = 78;
const KEY_U: i32 = 85;
const KEY_Y: i32 = 89;
const KEY_NUMPAD1: i32 = 97;
const KEY_NUMPAD2: i32 = 98;
const KEY_NUMPAD3: i32 = 99;
const KEY_NUMPAD4: i32 = 100;
const KEY_NUMPAD5: i32 = 101;
const KEY_NUMPAD6: i32 = 102;
const KEY_NUMPAD7: i32 = 103;
const KEY_NUMPAD8: i32 = 104;
const KEY_NUMPAD9: i32 = 105;
const KEY_DECIMAL: i32 = 110;

const WORLD_SIZE_X: i32 = 55;
const WORLD_SIZE_Y: i32 = 44;

type Coord = (i32, i32);

struct World {
	size_x: i32,
	size_y: i32,
	player_position: Coord,
	trees: Vec<Coord>
}

static mut WORLD: Option<World> = None;

fn make_world(size_x: i32, size_y: i32, seed: u64) -> World {
	let mut random = Random::seed_from_u64(seed);
	World {
		size_x: size_x,
		size_y: size_y,
		player_position: (size_x / 2, size_y / 2),
		trees: make_trees(100, size_x, size_y, &mut random),
	}
}

fn make_trees(max_trees: usize, size_x: i32, size_y: i32, random: &mut Random) -> Vec<Coord> {
	let mut coord_set: HashSet<Coord> = HashSet::with_capacity(max_trees);
	for _ in 0..max_trees {
		let coord = (random.gen_range(0..size_x), random.gen_range(0..size_y));
		coord_set.insert(coord);
	}
	Vec::from_iter(coord_set)
}

fn make_rgb(r: u8, g: u8, b: u8) -> u32 {
	(0xff << 24) + ((r as u32) << 16) + ((g as u32) << 8) + (b as u32)
}

fn draw_world(world: &World, screen_size_x: i32, screen_size_y: i32) {
	let green = make_rgb(0, 174, 0);
	let gray = make_rgb(168, 168, 168);

	let offset_x = (screen_size_x - world.size_x * 16) / 2;
	let offset_y = (screen_size_y - world.size_y * 16) / 2;

	let put_tile = |tile_index, world_x, world_y, color| {
		let dest_x = world_x * 16 + offset_x;
		let dest_y = world_y * 16 + offset_y;
		let src_x = 16 * (tile_index % 16);
		let src_y = 16 * (15 - tile_index / 16);
		draw_tile(dest_x, dest_y, 16, 16, color, src_x, src_y);
	};

	for y in 0..world.size_y {
		for x in 0..world.size_x {
			put_tile(132, x, y, green); // grass
		}
	}

	for (x, y) in &world.trees {
		put_tile(144, *x, *y, green);
	}

	put_tile(208, world.player_position.0, world.player_position.1, gray);
}

fn update_world(world: &mut World, key: i32, ctrl_key_down: bool, shift_key_down: bool) {
	let vertical_offset =
		if ctrl_key_down {-1} else {0} +
		if shift_key_down {1} else {0};

	let modifier = match key {
		KEY_LEFT => (-1, vertical_offset),
		KEY_UP => (0, 1),
		KEY_RIGHT => (1, vertical_offset),
		KEY_DOWN => (0, -1),
		KEY_B => (-1, -1),
		KEY_H => (-1, 0),
		KEY_J => (0, -1),
		KEY_K => (0, 1),
		KEY_L => (1, 0),
		KEY_N => (1, -1),
		KEY_U => (1, 1),
		KEY_Y => (-1, 1),
		KEY_NUMPAD1 => (-1, -1),
		KEY_NUMPAD2 => (0, -1),
		KEY_NUMPAD3 => (1, -1),
		KEY_NUMPAD4 => (-1, 0),
		KEY_NUMPAD5 => (0, 0),
		KEY_NUMPAD6 => (1, 0),
		KEY_NUMPAD7 => (-1, 1),
		KEY_NUMPAD8 => (0, 1),
		KEY_NUMPAD9 => (1, 1),
		KEY_DECIMAL => (0, 0),
		_ => (0,0)
	};

	let new_position = (
		max(0, min(world.size_x - 1, world.player_position.0 + modifier.0)),
		max(0, min(world.size_y - 1, world.player_position.1 + modifier.1))
	);

	if new_position != world.player_position {
		world.player_position = new_position;
		invalidate_screen();
	}
}

// Javascript imports:

extern {
	fn js_draw_tile(dest_x: i32, dest_y: i32, size_x: i32, size_y: i32, color: u32, src_x: i32, src_y: i32);
	fn js_invalidate_screen();
}

fn draw_tile(dest_x: i32, dest_y: i32, size_x: i32, size_y: i32, color: u32, src_x: i32, src_y: i32) {
	unsafe { js_draw_tile(dest_x, dest_y, size_x, size_y, color, src_x, src_y); }
}

fn invalidate_screen() {
	unsafe { js_invalidate_screen(); }
}

// Javascript exports:

#[no_mangle]
pub fn rs_on_draw(screen_size_x: i32, screen_size_y: i32) {
	if let Some(world) = unsafe { &mut WORLD } {
		draw_world(&world, screen_size_x, screen_size_y);
	}
}

#[no_mangle]
pub fn rs_start(seed0: u32, seed1: u32) -> () {
	let world = make_world(WORLD_SIZE_X, WORLD_SIZE_Y, ((seed0 as u64) << 32) + (seed1 as u64));
	unsafe { WORLD = Some(world); }
}

#[no_mangle]
pub fn rs_on_key_down(key: i32, ctrl_key_down: i32, shift_key_down: i32) -> () {
	if let Some(world) = unsafe { &mut WORLD } {
		update_world(world, key, ctrl_key_down != 0, shift_key_down != 0);
	}
}
