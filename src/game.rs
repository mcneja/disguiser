use rand::{SeedableRng};
use std::cmp::{min, max};

use crate::color_preset;
use crate::coord::Coord;
use crate::fontdata;
use crate::engine;
use crate::speech_bubbles::{get_horizontal_extents, puts_proportional, new_popups, Popups};
use crate::cell_grid::{CellGrid, CellType, ItemKind, Map, Player, Random, make_player, tile_def};
use crate::guard::{GuardMode, Lines, guard_act_all, is_guard_at, new_lines};
use crate::random_map;

const BAR_HEIGHT: i32 = fontdata::LINE_HEIGHT + 2;
const BAR_BACKGROUND_COLOR: u32 = 0xff101010;

const TILE_SIZE: i32 = 16;

pub struct Game {
    random: Random,
    level: usize,
    map: Map,
    lines: Lines,
    popups: Popups,
    player: Player,
    see_all: bool,
    show_msgs: bool,
    show_help: bool,
    help_page: usize,
}

pub fn new_game(seed: u64) -> Game {
    let mut random = Random::seed_from_u64(seed);
    let level = 0;
    let mut map = random_map::generate_map(&mut random, level);
    let player = make_player(map.pos_start);
    let lines = new_lines();
    let popups = new_popups();

    update_map_visibility(&mut map, player.pos);

    Game {
        random,
        level,
        lines,
        popups,
        map,
        player,
        see_all: true,
        show_msgs: true,
        show_help: false,
        help_page: 0,
    }
}

fn restart_game(game: &mut Game) {
    game.level = 0;
    game.map = random_map::generate_map(&mut game.random, game.level);
    game.player = make_player(game.map.pos_start);
    game.show_msgs = true;
    game.show_help = false;
    game.popups = new_popups();

    update_map_visibility(&mut game.map, game.player.pos);
}

pub fn on_draw(game: &Game, screen_size_x: i32, screen_size_y: i32) {
    let map = &game.map;
    let items = &game.map.items;
    let player = &game.player;
    let guards = &game.map.guards;

    let map_size_x = map.cells.extents()[0];
    let map_size_y = map.cells.extents()[1];

    let view_offset = viewport_offset(
        Coord(0, BAR_HEIGHT),
        Coord(screen_size_x, screen_size_y - BAR_HEIGHT),
        Coord(map_size_x as i32, map_size_y as i32),
        player.pos);

    let put_tile = |tile_index: u32, world_x: i32, world_y: i32, color: u32| {
        let dest_x = world_x * TILE_SIZE + view_offset.0;
        let dest_y = world_y * TILE_SIZE + view_offset.1;
        draw_tile_by_index(tile_index, dest_x, dest_y, color);
    };

    let put_offset_tile = |tile_index: u32, world_x: i32, world_y: i32, color: u32, add_x: i32, add_y: i32| {
        let dest_x = world_x * TILE_SIZE + view_offset.0 + add_x;
        let dest_y = world_y * TILE_SIZE + view_offset.1 + add_y;
        draw_tile_by_index(tile_index, dest_x, dest_y, color);
    };

    for x in 0..map_size_x {
        for y in 0..map_size_y {
            let cell = &map.cells[[x, y]];
            if !cell.seen && !game.see_all {
                continue;
            }
            let tile = tile_def(cell.cell_type);
            let color = if cell.lit || tile.ignores_lighting {tile.color} else {color_preset::DARK_BLUE};
            put_tile(tile.glyph, x as i32, y as i32, color);
        }
    }

    for item in items {
        let cell = &map.cells[[item.pos.0 as usize, item.pos.1 as usize]];
        if !cell.seen && !game.see_all {
            continue;
        }
        let glyph = glyph_for_item(item.kind);
        let color = if cell.lit {color_for_item(item.kind)} else {color_preset::DARK_BLUE};
        put_tile(glyph, item.pos.0, item.pos.1, color);
    }

    {
        let glyph = 208;

        let lit = map.cells[[player.pos.0 as usize, player.pos.1 as usize]].lit;
        let noisy = player.noisy;
        let damaged = player.damaged_last_turn;
        let hidden = player.hidden(map);

        let color =
            if damaged {0xff0000ff}
            else if noisy {color_preset::LIGHT_CYAN}
            else if hidden {0xd0101010}
            else if lit {color_preset::LIGHT_GRAY}
            else {color_preset::LIGHT_BLUE};

        put_tile(glyph, player.pos.0, player.pos.1, color);
    }

    for guard in guards {
        let glyph =
            if guard.dir.1 > 0 {210}
            else if guard.dir.1 < 0 {212}
            else if guard.dir.0 > 0 {209}
            else if guard.dir.0 < 0 {211}
            else {212};

        let cell = &map.cells[[guard.pos.0 as usize, guard.pos.1 as usize]];
        
        let visible = game.see_all || cell.seen || guard.speaking;

        if !visible {
            let dpos = player.pos - guard.pos;
            if dpos.length_squared() > 36 {
                continue;
            }
        }

        let color =
            if !visible {
                color_preset::DARK_GRAY
            } else if guard.mode == GuardMode::Patrol && !guard.speaking && !cell.lit {
                color_preset::DARK_BLUE
            } else {
                color_preset::LIGHT_MAGENTA
            };

        put_tile(glyph, guard.pos.0, guard.pos.1, color);
    }

    for guard in guards {
        if let Some(glyph) = guard.overhead_icon(map, player, game.see_all) {
            put_offset_tile(glyph, guard.pos.0, guard.pos.1, color_preset::LIGHT_YELLOW, 0, 10);
        }
    }

/*
    if let Some(guard) = guards.first() {
        if guard.region_goal != INVALID_REGION {
            let distance_field = map.compute_distances_to_region(guard.region_goal);
            for x in 0..map_size_x {
                for y in 0..map_size_y {
                    let pos = Vector::new(x as f32, ((map_size_y - 1) - y) as f32);
                    let d = distance_field[[x, y]];
                    if d == 0 || d == INFINITE_COST {
                        continue;
                    }
                    let digit = (d % 10) + 48;
                    let band = d / 10;
                    let image = &tileset[digit];
                    let pos_px = offset_px + TILE_SIZE.times(pos);
                    let color = if band == 0 {color_preset::WHITE} else if band == 1 {color_preset::LIGHT_YELLOW} else {color_preset::DARK_GRAY};
                    window.draw(
                        &Rectangle::new(pos_px, image.area().size()),
                        Blended(&image, color),
                    )
                }
            }
        }
    }
*/

/*
    if let Some(guard) = guards.first() {
        let image = &tileset[255];
        if guard.region_prev != INVALID_REGION {

            let region = &map.patrol_regions[guard.region_prev];
            for x in region.pos_min.0 .. region.pos_max.0 {
                for y in region.pos_min.1 .. region.pos_max.1 {
                    let pos = Vector::new(x as f32, ((map_size_y - 1) as i32 - y) as f32);
                    let pos_px = offset_px + TILE_SIZE.times(pos);
                    let color = Color {r:1.0, g:0.0, b:0.0, a:0.25};
                    window.draw(
                        &Rectangle::new(pos_px, image.area().size()),
                        Blended(&image, color),
                    )
                }
            }
        }
        if guard.region_goal != INVALID_REGION {
            let region = &map.patrol_regions[guard.region_goal];
            for x in region.pos_min.0 .. region.pos_max.0 {
                for y in region.pos_min.1 .. region.pos_max.1 {
                    let pos = Vector::new(x as f32, ((map_size_y - 1) as i32 - y) as f32);
                    let pos_px = offset_px + TILE_SIZE.times(pos);
                    let color = Color {r:0.0, g:1.0, b:0.0, a:0.25};
                    window.draw(
                        &Rectangle::new(pos_px, image.area().size()),
                        Blended(&image, color),
                    )
                }
            }
        }
    }
*/

    if game.show_msgs {
        game.popups.draw(
            screen_size_x,
            screen_size_y,
            Coord(TILE_SIZE, TILE_SIZE),
            view_offset,
            game.player.pos
        );
    }

    if game.show_help {
        draw_help(screen_size_x, screen_size_y, game.help_page);
    }

    draw_top_status_bar(screen_size_x, screen_size_y, game);
    draw_bottom_status_bar(screen_size_x, screen_size_y, game);
}

fn viewport_offset(viewport_screen_min: Coord, viewport_screen_max: Coord, world_size: Coord, world_focus: Coord) -> Coord {
    let viewport_screen_size = viewport_screen_max - viewport_screen_min;
    let world_screen_size = Coord(TILE_SIZE, TILE_SIZE).mul_components(world_size);
    let world_focus = Coord(TILE_SIZE, TILE_SIZE).mul_components(world_focus) + Coord(TILE_SIZE / 2, TILE_SIZE / 2);

    let world_offset_x =
        if world_screen_size.0 <= viewport_screen_size.0 {
            (viewport_screen_size.0 - world_screen_size.0) / 2
        } else {
            min(0, max(viewport_screen_size.0 - world_screen_size.0, viewport_screen_size.0 / 2 - world_focus.0))
        };

    let world_offset_y =
        if world_screen_size.1 <= viewport_screen_size.1 {
            (viewport_screen_size.1 - world_screen_size.1) / 2
        } else {
            min(0, max(viewport_screen_size.1 - world_screen_size.1, viewport_screen_size.1 / 2 - world_focus.1))
        };

    viewport_screen_min + Coord(world_offset_x, world_offset_y)
}

fn glyph_for_item(kind: ItemKind) -> u32 {
    match kind {
        ItemKind::Chair => 148,
        ItemKind::Table => 146,
        ItemKind::Bush => 144,
        ItemKind::Coin => 158,
        ItemKind::DoorNS => 169,
        ItemKind::DoorEW => 167,
        ItemKind::PortcullisNS => 194,
        ItemKind::PortcullisEW => 194,
    }
}

fn color_for_item(kind: ItemKind) -> u32 {
    match kind {
        ItemKind::Chair => color_preset::DARK_BROWN,
        ItemKind::Table => color_preset::DARK_BROWN,
        ItemKind::Bush => color_preset::DARK_GREEN,
        ItemKind::Coin => color_preset::LIGHT_YELLOW,
        ItemKind::DoorNS => color_preset::DARK_BROWN,
        ItemKind::DoorEW => color_preset::DARK_BROWN,
        ItemKind::PortcullisNS => color_preset::LIGHT_GRAY,
        ItemKind::PortcullisEW => color_preset::LIGHT_GRAY,
    }
}

fn move_player(game: &mut Game, mut dx: i32, mut dy: i32) {
    let player = &mut game.player;

    // Can't move if you're dead.

    if player.health == 0 {
        return;
    }

    // Are we trying to exit the level?

    let pos_new = player.pos + Coord(dx, dy);

    if !on_level(&game.map.cells, pos_new) && game.map.all_seen() && game.map.all_loot_collected() {
        game.level += 1;
        game.map = random_map::generate_map(&mut game.random, game.level);

        game.player.pos = game.map.pos_start;
        game.player.dir = Coord(0, 0);
        game.player.gold = 0;
        game.player.noisy = false;
        game.player.damaged_last_turn = false;
        game.player.finished_level = false;
        game.player.turns_remaining_underwater = 0;

        update_map_visibility(&mut game.map, game.player.pos);

        engine::invalidate_screen();
        return;
    }

    if dx == 0 || dy == 0 {
        if blocked(&game.map, player.pos, pos_new) {
            return;
        }
    } else if blocked(&game.map, player.pos, pos_new) {
        if halts_slide(&game.map, pos_new) {
            return;
        } else {
            // Attempting to move diagonally; may be able to slide along a wall.

            let v_blocked = blocked(&game.map, player.pos, player.pos + Coord(dx, 0));
            let h_blocked = blocked(&game.map, player.pos, player.pos + Coord(0, dy));

            if v_blocked {
                if h_blocked {
                    return;
                }

                dx = 0;
            } else {
                if !h_blocked {
                    return;
                }

                dy = 0;
            }
        }
    }

    pre_turn(game);

    let dpos = Coord(dx, dy);
    game.player.dir = dpos;
    game.player.pos += dpos;
    game.player.gold += game.map.collect_loot_at(game.player.pos);

    // Generate movement noises.

    let cell_type = game.map.cells[[game.player.pos.0 as usize, game.player.pos.1 as usize]].cell_type;

    if dpos != Coord(0, 0) && cell_type == CellType::GroundWoodCreaky {
        make_noise(&mut game.map, &mut game.player, &mut game.popups, "\u{ab}creak\u{bb}");
    }

    advance_time(game);

    engine::invalidate_screen();
}

fn make_noise(map: &mut Map, player: &mut Player, popups: &mut Popups, noise: &'static str) {
    player.noisy = true;
    popups.noise(player.pos, noise);

    for guard in map.guards_in_earshot(player.pos, 75) {
        guard.hear_thief();
    }
}

fn halts_slide(map: &Map, pos: Coord) -> bool {
    if pos.0 < 0 || pos.0 >= map.cells.extents()[0] as i32 || pos.1 < 0 || pos.1 >= map.cells.extents()[1] as i32 {
        return false;
    }

    if is_guard_at(map, pos.0, pos.1) {
        return true;
    }

    false
}

fn pre_turn(game: &mut Game) {
    game.show_msgs = true;
    game.popups.clear();
    game.player.noisy = false;
    game.player.damaged_last_turn = false;
    game.player.dir = Coord(0, 0);
}

const DIRS: [Coord; 4] = [
    Coord(-1, 0),
    Coord(1, 0),
    Coord(0, -1),
    Coord(0, 1),
];

fn advance_time(game: &mut Game) {
    if game.map.cells[[game.player.pos.0 as usize, game.player.pos.1 as usize]].cell_type == CellType::GroundWater {
        if game.player.turns_remaining_underwater > 0 {
            game.player.turns_remaining_underwater -= 1;
        }
    } else {
        game.player.turns_remaining_underwater = 7;
    }

    guard_act_all(&mut game.random, game.see_all, &mut game.popups, &mut game.lines, &mut game.map, &mut game.player);

    update_map_visibility(&mut game.map, game.player.pos);

    if game.map.all_seen() && game.map.all_loot_collected() {
        game.player.finished_level = true;
    }
}

fn update_map_visibility(map: &mut Map, pos_viewer: Coord) {
    map.recompute_visibility(pos_viewer);

    for dir in &DIRS {
        let pos = pos_viewer + *dir;
        if !blocked(map, pos_viewer, pos) {
            map.recompute_visibility(pos);
        }
    }
}

fn on_level(map: &CellGrid, pos: Coord) -> bool {
    let size_x = map.extents()[0] as i32;
    let size_y = map.extents()[1] as i32;
    pos.0 >= 0 && pos.1 >= 0 && pos.0 < size_x && pos.1 < size_y
}

fn blocked(map: &Map, pos_old: Coord, pos_new: Coord) -> bool {
    if !on_level(&map.cells, pos_new) {
        return true;
    }

    let tile_type = map.cells[[pos_new.0 as usize, pos_new.1 as usize]].cell_type;
    let tile = tile_def(tile_type);

    if tile.blocks_player {
        return true;
    }

    if tile_type == CellType::OneWayWindowE && pos_new.0 <= pos_old.0 {
        return true;
    }

    if tile_type == CellType::OneWayWindowW && pos_new.0 >= pos_old.0 {
        return true;
    }

    if tile_type == CellType::OneWayWindowN && pos_new.1 <= pos_old.1 {
        return true;
    }

    if tile_type == CellType::OneWayWindowS && pos_new.1 >= pos_old.1 {
        return true;
    }

    if is_guard_at(map, pos_new.0, pos_new.1) {
        return true;
    }

    false
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
    } else if key == engine::KEY_SPACE {
        game.show_msgs = !game.show_msgs;
        engine::invalidate_screen();
    } else if let Some(dir) = dir_from_key(key, ctrl_key_down, shift_key_down) {
        move_player(game, dir.0, dir.1);
    } else if ctrl_key_down {
        match key {
            engine::KEY_A => {
                game.see_all = !game.see_all;
                engine::invalidate_screen();
            },
            engine::KEY_C => {
                game.map.mark_all_unseen();
                update_map_visibility(&mut game.map, game.player.pos);
                engine::invalidate_screen();
            },
            engine::KEY_R => {
                restart_game(game);
                engine::invalidate_screen();
            },
            engine::KEY_S => {
                game.map.mark_all_seen();
                engine::invalidate_screen();
            },
            _ => {}
        }
    }
}

fn dir_from_key(key: i32, ctrl_key_down: bool, shift_key_down: bool) -> Option<Coord> {
    let vertical_offset =
        if ctrl_key_down {-1} else {0} +
        if shift_key_down {1} else {0};

    match key {
        engine::KEY_LEFT => Some(Coord(-1, vertical_offset)),
        engine::KEY_UP | engine::KEY_NUMPAD8 | engine::KEY_K => Some(Coord(0, 1)),
        engine::KEY_RIGHT => Some(Coord(1, vertical_offset)),
        engine::KEY_DOWN | engine::KEY_NUMPAD2 | engine::KEY_J => Some(Coord(0, -1)),
        engine::KEY_NUMPAD1 | engine::KEY_B => Some(Coord(-1, -1)),
        engine::KEY_NUMPAD4 | engine::KEY_H => Some(Coord(-1, 0)),
        engine::KEY_NUMPAD6 | engine::KEY_L => Some(Coord(1, 0)),
        engine::KEY_NUMPAD3 | engine::KEY_N => Some(Coord(1, -1)),
        engine::KEY_NUMPAD9 | engine::KEY_U => Some(Coord(1, 1)),
        engine::KEY_NUMPAD7 | engine::KEY_Y => Some(Coord(-1, 1)),
        engine::KEY_NUMPAD5 | engine::KEY_PERIOD => Some(Coord(0, 0)),
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

// Tile-set drawing

fn draw_tile_by_index(tile_index: u32, dest_x: i32, dest_y: i32, color: u32) {
    const TEXTURE_INDEX: u32 = 0;
    let src_x = ((tile_index & 15) * 16) as i32;
    let src_y = (240 - (tile_index & !15)) as i32;
    engine::draw_tile(dest_x, dest_y, TILE_SIZE, TILE_SIZE, color, TEXTURE_INDEX, src_x, src_y);
}

// Status bars

fn draw_bottom_status_bar(screen_size_x: i32, _screen_size_y: i32, game: &Game) {
    engine::draw_rect(0, 0, screen_size_x, BAR_HEIGHT, BAR_BACKGROUND_COLOR);

    let y_base = 2;

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

    let player_underwater = game.map.cells[[game.player.pos.0 as usize, game.player.pos.1 as usize]].cell_type == CellType::GroundWater && game.player.turns_remaining_underwater > 0;

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

    let percent_seen: usize = game.map.percent_seen();

    {
        const COLOR: u32 = 0xffa0a0a0;
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

    let y_base = screen_size_y - BAR_HEIGHT + 2;

    const COLOR: u32 = 0xffffffff; // white

    if game.show_help {
        let msg = format!("Page {} of {}", game.help_page + 1, HELP_MESSAGES.len());
        let (x_min, x_max) = get_horizontal_extents(&msg);
        let x = screen_size_x - (8 + (x_max - x_min));

        puts_proportional(x, y_base, &msg, COLOR);
        puts_proportional(8, y_base, "Press left/right arrow keys to view help, or Esc to close", COLOR);
    } else {
        let msg =
            if game.player.health == 0 {
                format!("You are dead! Press Ctrl+R for a new game.")
            } else if game.player.finished_level {
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
"ThiefRL 2 (Web version: 2021 March 8)

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

    puts_proportional(box_min_x + MARGIN, box_min_y + BOX_SIZE_Y - (fontdata::LINE_HEIGHT + MARGIN), help_msg, TEXT_COLOR);
}
