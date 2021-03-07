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

const TILE_SIZE: i32 = 16;

type Coord = (i32, i32);

pub struct Game {
	size_x: i32,
	size_y: i32,
	player: Player,
	trees: Vec<Coord>,
	map: Map,
    level: usize,
	game_over: bool,
	finished_level: bool,
	show_help: bool,
	help_page: usize,
}

struct Player {
    pos: Coord,
    max_health: usize,
    health: usize,
    gold: usize,
    turns_remaining_underwater: usize,
}

struct Map {
    total_loot: usize,
}

pub fn new_game(seed: u64) -> Game {
	let mut random = Random::seed_from_u64(seed);
	Game {
		size_x: WORLD_SIZE_X,
		size_y: WORLD_SIZE_Y,
		player: Player {
			pos: (WORLD_SIZE_X / 2, WORLD_SIZE_Y / 2),
			max_health: 5,
			health: 5,
			gold: 0,
			turns_remaining_underwater: 0,
		},
		trees: make_trees(100, WORLD_SIZE_X, WORLD_SIZE_Y, &mut random),
		map: Map {
			total_loot: 1,
		},
		level: 0,
		game_over: false,
		finished_level: false,
		show_help: false,
		help_page: 0,
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

pub fn on_draw(game: &Game, screen_size_x: i32, screen_size_y: i32) {
	const GREEN: u32 = 0xff00ae00;
	const GRAY: u32 = 0xffa8a8a8;

	let offset_x = (screen_size_x - game.size_x * TILE_SIZE) / 2;
	let offset_y = (screen_size_y - game.size_y * TILE_SIZE) / 2;

	let put_tile = |tile_index, world_x, world_y, color| {
		let dest_x = world_x * TILE_SIZE + offset_x;
		let dest_y = world_y * TILE_SIZE + offset_y;
		draw_tile_by_index(tile_index, dest_x, dest_y, color);
	};

	for y in 0..game.size_y {
		for x in 0..game.size_x {
			put_tile(132, x, y, GREEN); // grass
		}
	}

	for (x, y) in &game.trees {
		put_tile(144, *x, *y, GREEN);
	}

	put_tile(208, game.player.pos.0, game.player.pos.1, GRAY);

	draw_top_status_bar(screen_size_x, screen_size_y, game);
	draw_bottom_status_bar(screen_size_x, screen_size_y, game);

	if game.show_help {
		draw_help(screen_size_x, screen_size_y, game.help_page);
	}
}

pub fn on_key_down(game: &mut Game, key: i32, ctrl_key_down: bool, shift_key_down: bool) {
	let handle_key = if game.show_help {
		on_key_down_help_mode
	} else {
		on_key_down_game_mode
	};

	handle_key(game, key, ctrl_key_down, shift_key_down);
}

fn on_key_down_game_mode(game: &mut Game, key: i32, ctrl_key_down: bool, shift_key_down: bool) {
	if key == engine::KEY_SLASH {
		game.show_help = true;
		engine::invalidate_screen();
	} else if !game.game_over {
		if let Some(dir) = dir_from_key(key, ctrl_key_down, shift_key_down) {
			let new_position = (
				max(0, min(game.size_x - 1, game.player.pos.0 + dir.0)),
				max(0, min(game.size_y - 1, game.player.pos.1 + dir.1))
			);
		
			if new_position != game.player.pos {
				game.player.pos = new_position;
				engine::invalidate_screen();
			}
		}
	}
}

fn dir_from_key(key: i32, ctrl_key_down: bool, shift_key_down: bool) -> Option<Coord> {
	let vertical_offset =
		if ctrl_key_down {-1} else {0} +
		if shift_key_down {1} else {0};

	match key {
		engine::KEY_LEFT => Some((-1, vertical_offset)),
		engine::KEY_UP | engine::KEY_NUMPAD8 | engine::KEY_K => Some((0, 1)),
		engine::KEY_RIGHT => Some((1, vertical_offset)),
		engine::KEY_DOWN | engine::KEY_NUMPAD2 | engine::KEY_J => Some((0, -1)),
		engine::KEY_NUMPAD1 | engine::KEY_B => Some((-1, -1)),
		engine::KEY_NUMPAD4 | engine::KEY_H => Some((-1, 0)),
		engine::KEY_NUMPAD6 | engine::KEY_L => Some((1, 0)),
		engine::KEY_NUMPAD3 | engine::KEY_N => Some((1, -1)),
		engine::KEY_NUMPAD9 | engine::KEY_U => Some((1, 1)),
		engine::KEY_NUMPAD7 | engine::KEY_Y => Some((-1, 1)),
		engine::KEY_NUMPAD5 | engine::KEY_DECIMAL => Some((0, 0)),
		_ => None
	}
}

fn on_key_down_help_mode(game: &mut Game, key: i32, ctrl_key_down: bool, _shift_key_down: bool) {
	if ctrl_key_down {
		return;
	}

	match key {
		engine::KEY_ESCAPE | engine::KEY_SLASH => {
			game.show_help = false;
			engine::invalidate_screen();
		},
		engine::KEY_LEFT | engine::KEY_NUMPAD4 => {
			if game.help_page > 0 {
				game.help_page -= 1;
				engine::invalidate_screen();
			}
		},
		engine::KEY_RIGHT | engine::KEY_NUMPAD6 => {
			if game.help_page < HELP_MESSAGES.len() - 1 {
				game.help_page += 1;
				engine::invalidate_screen();
			}
		}
		_ => {}
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

fn get_horizontal_extents(s: &str) -> (i32, i32) {
    let mut x_min = std::i32::MAX;
    let mut x_max = std::i32::MIN;
    let mut x = 0;

    for c in s.chars() {
        if let Some(glyph) = glyph_lookup(c) {
            x_min = min(x_min, x + glyph.x_offset);
            x_max = max(x_max, x + glyph.x_offset + glyph.width);
            x += glyph.x_advance;
        }
    }

    (x_min, x_max)
}

// Tile-set drawing

fn draw_tile_by_index(tile_index: u32, dest_x: i32, dest_y: i32, color: u32) {
	const TEXTURE_INDEX: u32 = 0;
	const TILES_PER_ROW: u32 = 16; // 256 pixels wide divided by 16 pixels per tile
	let src_x = TILE_SIZE * (tile_index % TILES_PER_ROW) as i32;
	let src_y = TILE_SIZE * (tile_index / TILES_PER_ROW) as i32;
	engine::draw_tile(dest_x, dest_y, TILE_SIZE, TILE_SIZE, color, TEXTURE_INDEX, src_x, src_y);
}

// Status bars

fn draw_bottom_status_bar(screen_size_x: i32, _screen_size_y: i32, game: &Game) {
	engine::draw_rect(0, 0, screen_size_x, BAR_HEIGHT, BAR_BACKGROUND_COLOR);

    let y_base = 0;

    const HEALTH_COLOR: u32 = 0xff0000a8;
    let mut x = 8;
    x = puts_proportional(x, y_base, "Health", HEALTH_COLOR);
    x += 12;

    const TILE_HEALTHY: u32 = 213;
    for _ in 0..game.player.health {
		draw_tile_by_index(TILE_HEALTHY, x, y_base + 5, HEALTH_COLOR);
        x += TILE_SIZE;
    }

    const TILE_UNHEALTHY: u32 = 7;
    for _ in game.player.health..game.player.max_health {
		draw_tile_by_index(TILE_UNHEALTHY, x, y_base + 5, HEALTH_COLOR);
        x += TILE_SIZE;
    }

    let player_underwater = false; // game.map.cells[[game.player.pos.0 as usize, game.player.pos.1 as usize]].cell_type == CellType::GroundWater && game.player.turns_remaining_underwater > 0;

    if player_underwater {
        x = screen_size_x / 4 - 16;
        x = puts_proportional(x, y_base, "Air", AIR_COLOR);
        x += 8;

        const TILE_AIR: u32 = 214;
        const AIR_COLOR: u32 = 0xfffefe54;
        for _ in 0..game.player.turns_remaining_underwater - 1 {
			draw_tile_by_index(TILE_AIR, x, y_base + 5, AIR_COLOR);
            x += TILE_SIZE;
        }

        const TILE_NO_AIR: u32 = 7;
        const NO_AIR_COLOR: u32 = 0xffa8a800;
        for _ in game.player.turns_remaining_underwater - 1 .. 5 {
			draw_tile_by_index(TILE_NO_AIR, x, y_base + 5, NO_AIR_COLOR);
            x += TILE_SIZE;
        }
    }

    // Draw the tallies of what's been seen and collected.

    let percent_seen: usize = 0; // game.map.percent_seen();

    {
        const COLOR: u32 = 0xff363636;
        let seen_msg = format!("Level {}: {}% Seen", game.level + 1, percent_seen);
        let (x_min, x_max) = get_horizontal_extents(&seen_msg);
        let x = (screen_size_x - (x_max - x_min)) / 2;
        puts_proportional(x, y_base, &seen_msg, COLOR);
    }

    {
        const COLOR: u32 = 0xff36fefe;
        let loot_msg =
            if percent_seen < 100 {
                format!("Loot {}/?", game.player.gold)
            } else {
                format!("Loot {}/{}", game.player.gold, game.map.total_loot)
            };
        let (x_min, x_max) = get_horizontal_extents(&loot_msg);
        let x = screen_size_x - (8 + (x_max - x_min));
        puts_proportional(x, y_base, &loot_msg, COLOR);
    }
}

fn draw_top_status_bar(screen_size_x: i32, screen_size_y: i32, game: &Game) {
	engine::draw_rect(0, screen_size_y - BAR_HEIGHT, screen_size_x, BAR_HEIGHT, BAR_BACKGROUND_COLOR);

    let y_base = screen_size_y - BAR_HEIGHT + 7;

	const COLOR: u32 = 0xffffffff; // white

    if game.show_help {
		let msg = format!("Page {} of {}", game.help_page + 1, HELP_MESSAGES.len());
        let (x_min, x_max) = get_horizontal_extents(&msg);
        let x = screen_size_x - (8 + (x_max - x_min));

        puts_proportional(x, y_base, &msg, COLOR);
		puts_proportional(8, y_base, "Press left/right arrow keys to view help, or Esc to close", COLOR);
    } else {
        let msg =
            if game.game_over || game.player.health == 0 {
                format!("You are dead! Press Ctrl+N for a new game or Ctrl+R to restart.")
            } else if game.finished_level {
                format!("Level {} complete! Move off the edge of the map to advance to the next level.", game.level + 1)
            } else if game.level == 0 {
                format!("Welcome to level {}. Collect the gold coins and reveal the whole mansion. (Press ? for help.)", game.level + 1)
            } else if game.level == 1 {
                format!("Welcome to level {}. Watch out for the patrolling guard! (Press ? for help.)", game.level + 1)
            } else {
                format!("Press ? for help")
            };

        puts_proportional(8, y_base, &msg, COLOR);
    }
}

static HELP_MESSAGES: &[&str] = &[

// Page 1
"ThiefRL 2 (Web version: 2021 March 7)

Press right arrow for hints, or ? to toggle this help

Sneak into mansions, map them, steal all the loot and get out.

The guards cannot be injured! They also cannot cut corners diagonally.

Use the numpad keys to move horizontally, vertically, and diagonally.
Use numpad 5 to wait. Alternatively use the keys (H J K L Y U B N),
or arrow keys with Shift/Ctrl plus Left/Right to move diagonally.

Health is shown on the status bar in the lower left.

A 2016 Seven-day Roguelike Challenge game by James McNeill

Testing: Mike Gaffney, Mendi Carroll
Special Thanks: Mendi Carroll

http://playtechs.blogspot.com",

// Page 2
"Hints

Pick up gold coins by moving over them.

Diagonal movement is critical! Guards cannot cut corners, so moving
diagonally around corners is the key to gaining distance from them.

Guards can only see ahead of themselves.

If a guard sees you and is standing next to you, he will attack!

Bushes, tables, and water can all serve as hiding places. Patrolling guards
cannot see you when you are hidden. Alert guards (with a question mark
over their heads) can see you if they are next to you.

High one-way windows allow for quick escapes. Guards can't use them!

Guards can't see as far in the dark outside the mansion."
];

fn draw_help(screen_size_x: i32, screen_size_y: i32, help_page: usize) {
    const BOX_SIZE_X: i32 = 664;
    const BOX_SIZE_Y: i32 = 470;
    const MARGIN: i32 = 24;

    const SCREEN_DARKENING_COLOR: u32 = 0xa0101010;
    const WINDOW_BACKGROUND_COLOR: u32 = 0xff404040;
	const TEXT_COLOR: u32 = 0xffffffff;

    let box_min_x = (screen_size_x - BOX_SIZE_X) / 2;
    let box_min_y = (screen_size_y - (BAR_HEIGHT + BOX_SIZE_Y)) / 2 + BAR_HEIGHT;

	engine::draw_rect(0, BAR_HEIGHT, screen_size_x, screen_size_y - 2 * BAR_HEIGHT, SCREEN_DARKENING_COLOR);
	engine::draw_rect(box_min_x, box_min_y, BOX_SIZE_X, BOX_SIZE_Y, WINDOW_BACKGROUND_COLOR);

    let help_msg = HELP_MESSAGES[help_page];

    puts_proportional(box_min_x + MARGIN, box_min_y + BOX_SIZE_Y + 5 - (fontdata::LINE_HEIGHT + MARGIN), help_msg, TEXT_COLOR);
}
