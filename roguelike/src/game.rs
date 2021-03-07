use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::iter::FromIterator;
use std::cmp::min;
use std::cmp::max;

use crate::fontdata;
use crate::engine;

type Random = rand_pcg::Pcg32;

const WORLD_SIZE_X: i32 = 55;
const WORLD_SIZE_Y: i32 = 44;

const BAR_HEIGHT: i32 = fontdata::LINE_HEIGHT + 2;
const BAR_BACKGROUND_COLOR: u32 = 0xff101010;

type Coord = (i32, i32);

pub struct World {
	size_x: i32,
	size_y: i32,
	player_position: Coord,
	trees: Vec<Coord>
}

pub fn new_game(seed: u64) -> World {
	let mut random = Random::seed_from_u64(seed);
	World {
		size_x: WORLD_SIZE_X,
		size_y: WORLD_SIZE_Y,
		player_position: (WORLD_SIZE_X / 2, WORLD_SIZE_Y / 2),
		trees: make_trees(100, WORLD_SIZE_X, WORLD_SIZE_Y, &mut random),
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

pub fn on_draw(world: &World, screen_size_x: i32, screen_size_y: i32) {
	let green = make_rgb(0, 174, 0);
	let gray = make_rgb(168, 168, 168);

	const TILE_SIZE: i32 = 16;
	const TILES_PER_ROW: i32 = 16; // 256 pixels wide divided by 16 pixels per tile

	let offset_x = (screen_size_x - world.size_x * TILE_SIZE) / 2;
	let offset_y = (screen_size_y - world.size_y * TILE_SIZE) / 2;

	let put_tile = |tile_index, world_x, world_y, color| {
		let dest_x = world_x * TILE_SIZE + offset_x;
		let dest_y = world_y * TILE_SIZE + offset_y;
		let src_x = TILE_SIZE * (tile_index % TILES_PER_ROW);
		let src_y = TILE_SIZE * (tile_index / TILES_PER_ROW);
		let texture_index = 0;
		engine::draw_tile(dest_x, dest_y, TILE_SIZE, TILE_SIZE, color, texture_index, src_x, src_y);
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

	engine::draw_rect(0, screen_size_y - BAR_HEIGHT, screen_size_x, BAR_HEIGHT, BAR_BACKGROUND_COLOR);
	engine::draw_rect(0, 0, screen_size_x, BAR_HEIGHT, BAR_BACKGROUND_COLOR);

	puts_proportional(4, screen_size_y - fontdata::LINE_HEIGHT + 4, "Press F1 for help", 0xffffffff);
}

pub fn on_key_down(world: &mut World, key: i32, ctrl_key_down: bool, shift_key_down: bool) {
	let vertical_offset =
		if ctrl_key_down {-1} else {0} +
		if shift_key_down {1} else {0};

	let modifier = match key {
		engine::KEY_LEFT => (-1, vertical_offset),
		engine::KEY_UP => (0, 1),
		engine::KEY_RIGHT => (1, vertical_offset),
		engine::KEY_DOWN => (0, -1),
		engine::KEY_B => (-1, -1),
		engine::KEY_H => (-1, 0),
		engine::KEY_J => (0, -1),
		engine::KEY_K => (0, 1),
		engine::KEY_L => (1, 0),
		engine::KEY_N => (1, -1),
		engine::KEY_U => (1, 1),
		engine::KEY_Y => (-1, 1),
		engine::KEY_NUMPAD1 => (-1, -1),
		engine::KEY_NUMPAD2 => (0, -1),
		engine::KEY_NUMPAD3 => (1, -1),
		engine::KEY_NUMPAD4 => (-1, 0),
		engine::KEY_NUMPAD5 => (0, 0),
		engine::KEY_NUMPAD6 => (1, 0),
		engine::KEY_NUMPAD7 => (-1, 1),
		engine::KEY_NUMPAD8 => (0, 1),
		engine::KEY_NUMPAD9 => (1, 1),
		engine::KEY_DECIMAL => (0, 0),
		_ => (0,0)
	};

	let new_position = (
		max(0, min(world.size_x - 1, world.player_position.0 + modifier.0)),
		max(0, min(world.size_y - 1, world.player_position.1 + modifier.1))
	);

	if new_position != world.player_position {
		world.player_position = new_position;
		engine::invalidate_screen();
	}
}

// Text rendering stuff (temporarily here)

fn glyph_lookup(c: char) -> Option<&'static fontdata::Glyph> {
    let id = c as usize;
    fontdata::GLYPH.iter().find(|&glyph| glyph.id == id)
}

fn puts_proportional(mut x: i32, mut y: i32, s: &str, color: u32) -> i32 {
	let x_base = x;
	const TEXTURE_INDEX: u32 = 1;

    for c in s.chars() {
        if c == '\n' {
            y -= if x == x_base {fontdata::LINE_HEIGHT / 2} else {fontdata::LINE_HEIGHT};
            x = x_base;
        } else if let Some(glyph) = glyph_lookup(c) {
			engine::draw_tile(x + glyph.x_offset, y + glyph.y_offset, glyph.width, glyph.height, color, TEXTURE_INDEX, glyph.x, glyph.y);
            x += glyph.x_advance;
        }
    }

    x
}
