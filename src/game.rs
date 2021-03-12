use rand::SeedableRng;
use std::cmp::{min, max};

use crate::cell_grid::{CellGrid, CellType, ItemKind, Map, Player, Random, make_player, tile_def};
use crate::color_preset;
use crate::coord::Coord;
use crate::engine;
use crate::fontdata;
use crate::guard::{GuardMode, Lines, guard_act_all, new_lines, update_dir};
use crate::random_map;
use crate::speech_bubbles::{get_horizontal_extents, puts_proportional, new_popups, Popups};

const BAR_HEIGHT: i32 = fontdata::LINE_HEIGHT + 2;
const BAR_BACKGROUND_COLOR: u32 = 0xff101010;

const TILE_SIZE: i32 = 16;

const INITIAL_LEVEL: usize = 0;
const SEE_ALL_DEFAULT: bool = false;

pub struct Game {
    random: Random,
    level: usize,
    map: Map,
    lines: Lines,
    popups: Popups,
    player: Player,
    finished_level: bool,
    see_all: bool,
    show_msgs: bool,
    show_help: bool,
    help_page: usize,
}

pub fn new_game(seed: u64) -> Game {
    let mut random = Random::seed_from_u64(seed);
    let level = INITIAL_LEVEL;
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
        finished_level: false,
        see_all: SEE_ALL_DEFAULT,
        show_msgs: true,
        show_help: false,
        help_page: 0,
    }
}

fn restart_game(game: &mut Game) {
    game.level = INITIAL_LEVEL;
    game.map = random_map::generate_map(&mut game.random, game.level);
    game.finished_level = false;
    game.player = make_player(game.map.pos_start);
    game.show_msgs = true;
    game.show_help = false;
    game.popups = new_popups();

    update_map_visibility(&mut game.map, game.player.pos);
}

fn finished_level(map: &Map) -> bool {
    map.all_loot_collected() && map.all_seen()
}

pub fn on_draw(game: &Game, screen_size_x: i32, screen_size_y: i32) {
    let map = &game.map;
    let items = &game.map.items;
    let player = &game.player;
    let guards = &game.map.guards;

    let map_size_x = map.cells.extents()[0];
    let map_size_y = map.cells.extents()[1];

    let view_min = Coord(0, BAR_HEIGHT);
    let view_max = Coord(screen_size_x, screen_size_y - BAR_HEIGHT);
    let view_offset = viewport_offset(
        view_min,
        view_max,
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

    // Base map

    const UNLIT_COLOR: u32 = color_preset::LIGHT_BLUE;

    for x in 0..map_size_x {
        for y in 0..map_size_y {
            let cell = &map.cells[[x, y]];
            if !cell.seen && !game.see_all {
                continue;
            }
            let tile = tile_def(cell.cell_type);
            let color = if cell.lit || tile.ignores_lighting {tile.color} else {UNLIT_COLOR};
            put_tile(tile.glyph, x as i32, y as i32, color);
        }
    }

    // Items

    for item in items {
        let cell = &map.cells[[item.pos.0 as usize, item.pos.1 as usize]];
        if !cell.seen && !game.see_all {
            continue;
        }
        let glyph = glyph_for_item(item.kind);
        let color = if cell.lit {color_for_item(item.kind)} else {UNLIT_COLOR};
        put_tile(glyph, item.pos.0, item.pos.1, color);
    }

    // Halo around player

    /*
    {
        let pos = player.pos * TILE_SIZE + view_offset + Coord(-8, -8);
        let color = 0x40ffffff;
        draw_tile_by_index(228, pos.0, pos.1, color);
        draw_tile_by_index(229, pos.0 + TILE_SIZE, pos.1, color);
        draw_tile_by_index(230, pos.0, pos.1 + TILE_SIZE, color);
        draw_tile_by_index(231, pos.0 + TILE_SIZE, pos.1 + TILE_SIZE, color);
    }
    */

    // Pointers at player along map edges

    /*
    {
        let pos = player.pos * TILE_SIZE + view_offset;
        let view_edge_min = Coord(max(view_offset.0, view_min.0), max(view_offset.1, view_min.1));
        let view_edge_max = Coord(min((map_size_x as i32) * TILE_SIZE + view_offset.0, view_max.0), min((map_size_y as i32) * TILE_SIZE + view_offset.1, view_max.1));
        let color = 0x40ffffff;

        draw_tile_by_index(232, view_edge_min.0, pos.1, color);
        draw_tile_by_index(233, view_edge_max.0 - TILE_SIZE, pos.1, color);
        draw_tile_by_index(234, pos.0, view_edge_min.1, color);
        draw_tile_by_index(235, pos.0, view_edge_max.1 - TILE_SIZE, color);
    }
    */

    // Player

    {
        let tile_index = tile_index_offset_for_dir(player.dir) + if player.disguised {212} else {208};

        let lit = map.cells[[player.pos.0 as usize, player.pos.1 as usize]].lit;
        let hidden = player.hidden(map);

        let color =
            if player.damaged_last_turn {0xff0000ff}
            else if player.noisy {color_preset::LIGHT_CYAN}
            else if hidden {0xd0101010}
            else if !lit {color_preset::LIGHT_BLUE}
            else if player.disguised {color_preset::LIGHT_MAGENTA}
            else {color_preset::LIGHT_GRAY};

        put_tile(tile_index, player.pos.0, player.pos.1, color);
    }

    // Guards

    for guard in guards {
        let tile_index = 212 + tile_index_offset_for_dir(guard.dir);
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
                UNLIT_COLOR
            } else {
                color_preset::LIGHT_MAGENTA
            };

        put_tile(tile_index, guard.pos.0, guard.pos.1, color);
    }

    // Guard overhead icons

    for guard in guards {
        if let Some((tile_index, color)) = guard.overhead_icon_and_color(map, player, game.see_all) {
            put_offset_tile(tile_index, guard.pos.0, guard.pos.1, color, 0, 10);
        }
    }

    // Player overhead icon

    put_offset_tile(218, player.pos.0, player.pos.1, color_preset::LIGHT_YELLOW, 0, 10);

    // Draw a guard's distance field to goal region

    /*
    if let Some(guard) = guards.first() {
        if guard.region_goal != crate::cell_grid::INVALID_REGION {
            let distance_field = map.compute_distances_to_region(guard.region_goal);
            for x in 0..map_size_x {
                for y in 0..map_size_y {
                    let d = distance_field[[x, y]];
                    if d == 0 || d == crate::cell_grid::INFINITE_COST {
                        continue;
                    }
                    let digit = (d % 10) + 48;
                    let band = d / 10;
                    let color = if band == 0 {color_preset::WHITE} else if band == 1 {color_preset::LIGHT_YELLOW} else {color_preset::DARK_GRAY};
                    put_tile(digit as u32, x as i32, y as i32, color);
                }
            }
        }
    }
    */

    // Highlight a guard's previous and goal regions

    /*
    if let Some(guard) = guards.first() {
        if guard.region_prev != crate::cell_grid::INVALID_REGION {
            const COLOR: u32 = 0x400000ff;
            let region = &map.patrol_regions[guard.region_prev];
            let pos = view_offset + region.pos_min * TILE_SIZE;
            let size = (region.pos_max - region.pos_min) * TILE_SIZE;
            engine::draw_rect(pos.0, pos.1, size.0, size.1, COLOR);
        }
        if guard.region_goal != crate::cell_grid::INVALID_REGION {
            const COLOR: u32 = 0x4000ff00;
            let region = &map.patrol_regions[guard.region_goal];
            let pos = view_offset + region.pos_min * TILE_SIZE;
            let size = (region.pos_max - region.pos_min) * TILE_SIZE;
            engine::draw_rect(pos.0, pos.1, size.0, size.1, COLOR);
        }
    }
    */

    // Speech bubbles and sounds

    if game.show_msgs {
        game.popups.draw(
            screen_size_x,
            screen_size_y,
            Coord(TILE_SIZE, TILE_SIZE),
            view_offset,
            game.player.pos
        );
    }

    // Help and status

    if game.show_help {
        draw_help(screen_size_x, screen_size_y, game.help_page);
    }

    draw_top_status_bar(screen_size_x, screen_size_y, game);
    draw_bottom_status_bar(screen_size_x, screen_size_y, game);
}

fn tile_index_offset_for_dir(dir: Coord) -> u32 {
    if dir.1 > 0 {1}
    else if dir.1 < 0 {3}
    else if dir.0 > 0 {0}
    else if dir.0 < 0 {2}
    else {3}
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
        ItemKind::Outfit1 => 163,
        ItemKind::Outfit2 => 163,
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
        ItemKind::Outfit1 => color_preset::LIGHT_GRAY,
        ItemKind::Outfit2 => color_preset::LIGHT_MAGENTA,
    }
}

fn advance_to_next_level(game: &mut Game) {
    game.level += 1;
    game.map = random_map::generate_map(&mut game.random, game.level);
    game.finished_level = false;

    game.player.pos = game.map.pos_start;
    game.player.dir = Coord(0, -1);
    game.player.gold = 0;
    game.player.noisy = false;
    game.player.disguised = false;
    game.player.damaged_last_turn = false;
    game.player.turns_remaining_underwater = 0;

    update_map_visibility(&mut game.map, game.player.pos);

    engine::invalidate_screen();
}

fn move_player(game: &mut Game, mut dpos: Coord) {
    let player = &mut game.player;

    // Can't move if you're dead.

    if player.health == 0 {
        return;
    }

    // Are we trying to exit the level?

    let pos_new = player.pos + dpos;

    if !on_level(&game.map.cells, pos_new) && finished_level(&game.map) {
        advance_to_next_level(game);
        return;
    }

    if blocked(&game.map, player.pos, pos_new) {
        if dpos.0 == 0 || dpos.1 == 0 || halts_slide(&game.map, pos_new) {
            try_use_in_direction(game, dpos);
            return;
        } else {
            // Attempting to move diagonally; may be able to slide along a wall.

            let v_blocked = blocked(&game.map, player.pos, player.pos + Coord(dpos.0, 0));
            let h_blocked = blocked(&game.map, player.pos, player.pos + Coord(0, dpos.1));

            if v_blocked {
                if h_blocked {
                    try_use_in_direction(game, dpos);
                    return;
                }
                dpos.0 = 0;
            } else {
                if !h_blocked {
                    try_use_in_direction(game, dpos);
                    return;
                }
                dpos.1 = 0;
            }
        }
    }

    pre_turn(game);

    game.player.dir = update_dir(game.player.dir, dpos);
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

fn try_use_in_direction(game: &mut Game, dpos: Coord) {
    let pos = game.player.pos + dpos;
    if let Some(outfit_new) = game.map.try_use_outfit_at(pos, if game.player.disguised {ItemKind::Outfit2} else {ItemKind::Outfit1}) {
        pre_turn(game);
        game.player.disguised = outfit_new != ItemKind::Outfit1;
        game.player.dir = update_dir(game.player.dir, dpos);
        advance_time(game);
        engine::invalidate_screen();
    }
}

fn make_noise(map: &mut Map, player: &mut Player, popups: &mut Popups, noise: &'static str) {
    player.noisy = true;
    popups.noise(player.pos, noise);

    for guard in map.guards_in_earshot(player.pos, 75) {
        guard.hear_thief();
    }
}

fn halts_slide(map: &Map, pos: Coord) -> bool {
    if !on_level(&map.cells, pos) {
        return false;
    }

    if map.is_guard_at(pos) {
        return true;
    }

    if map.is_outfit_at(pos) {
        return true;
    }

    false
}

fn pre_turn(game: &mut Game) {
    game.show_msgs = true;
    game.popups.clear();
    game.player.noisy = false;
    game.player.damaged_last_turn = false;
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

    if finished_level(&game.map) {
        game.finished_level = true;
    }
}

fn update_map_visibility(map: &mut Map, pos_viewer: Coord) {
    map.recompute_visibility(pos_viewer);

    for dir in &DIRS {
        let pos = pos_viewer + *dir;
        if map.player_can_see_in_direction(pos_viewer, *dir) {
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

    if pos_old == pos_new {
        return false;
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

    if map.is_guard_at(pos_new) {
        return true;
    }

    if map.is_outfit_at(pos_new) {
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
        move_player(game, dir);
    } else if ctrl_key_down {
        match key {
            engine::KEY_A => {
                game.see_all = !game.see_all;
                engine::invalidate_screen();
            },
            engine::KEY_C => {
                game.map.mark_all_unseen();
                update_map_visibility(&mut game.map, game.player.pos);
                game.finished_level = finished_level(&game.map);
                engine::invalidate_screen();
            },
            engine::KEY_D => {
                game.player.disguised = !game.player.disguised;
                engine::invalidate_screen();
            },
            engine::KEY_L => {
                game.player.gold += game.map.collect_all_loot();
                game.finished_level = finished_level(&game.map);
                engine::invalidate_screen();
            },
            engine::KEY_R => {
                restart_game(game);
                engine::invalidate_screen();
            },
            engine::KEY_S => {
                game.map.mark_all_seen();
                game.finished_level = finished_level(&game.map);
                engine::invalidate_screen();
            },
            _ => {}
        }
    }
}

fn dir_from_key(key: i32, ctrl_key_down: bool, shift_key_down: bool) -> Option<Coord> {
    if ctrl_key_down || shift_key_down {
        let vertical_offset =
            if ctrl_key_down {-1} else {0} +
            if shift_key_down {1} else {0};
        match key {
            engine::KEY_LEFT => Some(Coord(-1, vertical_offset)),
            engine::KEY_UP => Some(Coord(0, 1)),
            engine::KEY_RIGHT => Some(Coord(1, vertical_offset)),
            engine::KEY_DOWN => Some(Coord(0, -1)),
            _ => None
        }
    } else {
        match key {
            engine::KEY_LEFT => Some(Coord(-1, 0)),
            engine::KEY_UP | engine::KEY_NUMPAD8 | engine::KEY_K => Some(Coord(0, 1)),
            engine::KEY_RIGHT => Some(Coord(1, 0)),
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

    const TILE_HEALTHY: u32 = 5;
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
"Disguiser

Press right arrow for hints, or ? to toggle this help

Sneak into mansions, map them, steal all the loot and get out.

The guards cannot be injured! They also cannot cut corners diagonally.

Use the numpad keys to move horizontally, vertically, and diagonally.
Use numpad 5 to wait. Alternatively use the keys (H J K L Y U B N .),
or arrow keys with Shift/Ctrl plus Left/Right to move diagonally.

Health is shown on the status bar in the lower left.

A 2021 Seven-day Roguelike Challenge game by James McNeill

Special Thanks: Mendi Carroll

mcneja.github.io
playtechs.blogspot.com",

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
