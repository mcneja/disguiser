use crate::color_preset;
use crate::coord::Coord;
use crate::fontdata::{Glyph, GLYPH, LINE_HEIGHT};
use crate::engine;

use std::cmp::{min, max};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PopupType {
    Noise,
    Damage,
    GuardSpeech,
    Narration,
}

pub struct Popup {
    pub popup_type: PopupType,
    pub world_origin: Coord, // world tile position
    pub msg: &'static str,
}

struct PopupPlaced {
    pub popup_type: PopupType,
    pub world_origin: Coord, // world tile position
    pub msg: &'static str,
    pub size: Coord, // 16x16 tiles
    pub offset: Coord, // pixels, from top left to first character position
    pub pos: Coord,
}

pub struct Popups {
    popups: Vec<Popup>
}

pub fn new_popups() -> Popups {
    Popups {
        popups: Vec::new()
    }
}

impl Popups {
    pub fn clear(&mut self) {
        self.popups.clear();
    }
    
    pub fn guard_speech(&mut self, pos: Coord, s: &'static str) {
        self.push(PopupType::GuardSpeech, pos, s);
    }

    pub fn damage(&mut self, pos: Coord, s: &'static str) {
        self.push(PopupType::Damage, pos, s);
    }

    pub fn noise(&mut self, pos: Coord, s: &'static str) {
        self.push(PopupType::Noise, pos, s);
    }

    pub fn draw(&self, screen_size_x: i32, screen_size_y: i32, view_scale: Coord, view_offset: Coord, focus: Coord) {
        // Compute the correct world area that is visible in the viewport?
        let view_min = Coord(0, 0);
        let view_max = Coord(screen_size_x, screen_size_y);
        let placed_popups = layout(view_min, view_max, focus, &self.popups);
        for p in &placed_popups {
            draw_popup(view_scale, view_offset, p);
        }
    }

    fn push(&mut self, popup_type: PopupType, pos: Coord, s: &'static str) {
        self.popups.push(
            Popup {
                popup_type: popup_type,
                world_origin: pos,
                msg: s
            }
        );
    }
}

fn text_color(popup_type: PopupType) -> u32 {
    match popup_type {
        PopupType::Noise => color_preset::LIGHT_CYAN,
        PopupType::Damage => color_preset::LIGHT_YELLOW,
        PopupType::GuardSpeech => color_preset::WHITE,
        PopupType::Narration => color_preset::BLACK,
    }
}

fn background_color(popup_type: PopupType) -> u32 {
    match popup_type {
        PopupType::Noise => color_preset::BLACK,
        PopupType::Damage => color_preset::BLACK,
        PopupType::GuardSpeech => color_preset::BLACK,
        PopupType::Narration => color_preset::WHITE,
    }
}

fn border_color(popup_type: PopupType) -> u32 {
    match popup_type {
        PopupType::Noise => color_preset::LIGHT_CYAN,
        PopupType::Damage => color_preset::LIGHT_YELLOW,
        PopupType::GuardSpeech => color_preset::LIGHT_MAGENTA,
        PopupType::Narration => color_preset::WHITE,
    }
}

fn has_border(popup_type: PopupType) -> bool {
    match popup_type {
        PopupType::Noise => false,
        PopupType::Damage => false,
        PopupType::GuardSpeech => true,
        PopupType::Narration => true,
    }
}

// This function and TILE_SIZE are duplicated from game.rs!

fn draw_tile_by_index(tile_index: u32, dest_x: i32, dest_y: i32, color: u32) {
    const TILE_SIZE: i32 = 16;
    const TEXTURE_INDEX: u32 = 0;
    let src_x = ((tile_index & 15) * 16) as i32;
    let src_y = (240 - (tile_index & !15)) as i32;
    engine::draw_tile(dest_x, dest_y, TILE_SIZE, TILE_SIZE, color, TEXTURE_INDEX, src_x, src_y);
}

fn draw_popup(view_scale: Coord, view_offset: Coord, p: &PopupPlaced) {
    let has_box = has_border(p.popup_type);

    let screen_from_world = |pos: Coord| { view_offset + view_scale.mul_components(pos) };

    let text_y_offset = -LINE_HEIGHT - p.offset.1 + if has_box {0} else {-16};
    let text_pos = screen_from_world(p.pos + Coord(0, p.size.1) + if has_box {Coord(1, -1)} else {Coord(0, 0)}) + Coord(0, text_y_offset);

    // Draw background and border.

    let box_pos = screen_from_world(p.pos);
    let box_size = view_scale.mul_components(p.size);

    let put_tile = |tile_index: u32, x: i32, y: i32, color: u32| {
        let pos = screen_from_world(Coord(x, y));
        draw_tile_by_index(tile_index, pos.0, pos.1, color);
    };

    let fill_rect = |tile_index: u32, min_x: i32, min_y: i32, size_x: i32, size_y: i32, color: u32| {
        for x in min_x..min_x+size_x {
            for y in min_y..min_y+size_y {
                put_tile(tile_index, x, y, color);
            }
        }
    };

    if has_box {
        engine::draw_rect(box_pos.0, box_pos.1, box_size.0, box_size.1, background_color(p.popup_type));

        let sx = p.size.0;
        let sy = p.size.1;
        let x0 = p.pos.0;
        let y0 = p.pos.1;
        let x1 = p.pos.0 + sx - 1;
        let y1 = p.pos.1 + sy - 1;
        let border_color = border_color(p.popup_type);

        put_tile(228, x0, y0, border_color);
        put_tile(229, x1, y0, border_color);
        put_tile(230, x0, y1, border_color);
        put_tile(231, x1, y1, border_color);
        fill_rect(226, x0 + 1, y0, sx-2, 1, border_color);
        fill_rect(227, x0 + 1, y1, sx-2, 1, border_color);
        fill_rect(224, x0, y0 + 1, 1, sy-2, border_color);
        fill_rect(225, x1, y0 + 1, 1, sy-2, border_color);
    } else {
        // Draw text "outline"
        puts_proportional(text_pos.0 + 2, text_pos.1 - 2, p.msg, color_preset::BLACK);
        puts_proportional(text_pos.0 + 1, text_pos.1 - 1, p.msg, color_preset::BLACK);
    }

    // Draw the text

    puts_proportional(text_pos.0, text_pos.1, p.msg, text_color(p.popup_type));
}

type Score = (i32, f32);

fn compute_score(
    view_min: Coord,
    view_max: Coord,
    origin: Coord,
    dir: Coord,
    pos: Coord,
    size: Coord
) -> Score {
    let unclipped_box_min = pos;
    let unclipped_box_max = pos + size;
    let clipped_box_min = Coord(max(view_min.0, unclipped_box_min.0), max(view_min.1, unclipped_box_min.1));
    let clipped_box_max = Coord(min(view_max.0, unclipped_box_max.0), min(view_max.1, unclipped_box_max.1));

    let clipped_box_area = max(0, clipped_box_max.0 - clipped_box_min.0) * max(0, clipped_box_max.1 - clipped_box_min.1);
    let unclipped_box_area = (unclipped_box_max.0 - unclipped_box_min.0) * (unclipped_box_max.1 - unclipped_box_min.1);

    let offscreen_area = unclipped_box_area - clipped_box_area;

    let dir2 = (pos - origin) * 2 + size;

    let dot = dir2.dot(dir) as f32 / (dir2.length_squared() as f32).sqrt();

    (offscreen_area, -dot)
}

fn size_and_offset(p: &Popup) -> (Coord, Coord) {

    // Very similar to get_horizontal_extents:

    let mut num_lines = 1;
    let mut x_min = std::i32::MAX;
    let mut x_max = std::i32::MIN;
    let mut x = 0;

    for c in p.msg.chars() {
        if c == '\n' {
            num_lines += 1;
            x = 0;
        }
        else if let Some(glyph) = glyph_lookup(c) {
            x_min = min(x_min, x + glyph.x_offset);
            x_max = max(x_max, x + glyph.x_offset + glyph.width);
            x += glyph.x_advance;
        }
    }

    let width = x_max - x_min;
    let height = num_lines * LINE_HEIGHT;

    const TILE_SCREEN_SIZE: i32 = 16;// * g_worldScale; // This needs to get plumbed in; it allows rendering at different sizes

    let size_internal = Coord(
        (width + TILE_SCREEN_SIZE - 1) / TILE_SCREEN_SIZE,
        (height + TILE_SCREEN_SIZE - 1) / TILE_SCREEN_SIZE
    );

    let offset = Coord(
        (TILE_SCREEN_SIZE * size_internal.0 - width) / 2 - x_min,
        (TILE_SCREEN_SIZE * size_internal.1 - height) / 2
    );

    // Non-noise text boxes have borders.

    let size = if has_border(p.popup_type) {
        size_internal + Coord(2, 2)
    } else {
        size_internal
    };

    (size, offset)
}

fn layout_single(view_min: Coord, view_max: Coord, focus: Coord, p: &Popup) -> PopupPlaced {

    let (size, offset) = size_and_offset(p);

    let mut pos = Coord(0, 0);

    if p.popup_type == PopupType::Narration {
        // Center narration.
        pos.0 = (view_min.0 + view_max.0 - size.0) / 2;
        pos.1 = (view_min.1 + view_max.1 - size.1) / 2;
    } else {
        // Search for a position that is on the opposite side of the source
        // from the focus, with a center position as close as possible to
        // the line between source and focus.

        let dir = if p.world_origin == focus { Coord(0, 1) } else { p.world_origin - focus };

        let mut score_best: Score = (10000, 1000.0);

        // Generate positions along the top

        for x in 0..size.0 {
            let pos_trial = p.world_origin + Coord(-x, 2);
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }

        // Generate positions along the bottom

        for x in 0..size.0 {
            let pos_trial = p.world_origin + Coord(-x, -size.1);
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }

        // Generate positions along the sides

        for y in 0..size.1 {
            let pos_trial = p.world_origin + Coord(-size.0, -y);
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }

        for y in 0..size.1 {
            let pos_trial = p.world_origin + Coord(1, -y);
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }
    }

    PopupPlaced {
        popup_type: p.popup_type,
        world_origin: p.world_origin,
        msg: p.msg,
        size: size,
        offset: offset,
        pos: pos,
    }
}

fn layout(view_min: Coord, view_max: Coord, focus: Coord, popups: &[Popup]) -> Vec<PopupPlaced> {
    popups.iter().map(|p| layout_single(view_min, view_max, focus, &p)).collect()
}

pub fn glyph_lookup(c: char) -> Option<&'static Glyph> {
    let id = c as usize;
    GLYPH.iter().find(|&glyph| glyph.id == id)
}

pub fn get_horizontal_extents(s: &str) -> (i32, i32) {
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

pub fn puts_proportional(mut x: i32, mut y: i32, s: &str, color: u32) -> i32 {
    let x_base = x;
    const TEXTURE_INDEX: u32 = 1;

    for c in s.chars() {
        if c == '\n' {
            y -= if x == x_base {LINE_HEIGHT / 2} else {LINE_HEIGHT};
            x = x_base;
        } else if let Some(glyph) = glyph_lookup(c) {
            engine::draw_tile(
                x + glyph.x_offset,
                y + LINE_HEIGHT - (glyph.height + glyph.y_offset),
                glyph.width,
                glyph.height,
                color,
                TEXTURE_INDEX,
                glyph.x,
                glyph.y);
            x += glyph.x_advance;
        }
    }

    x
}
