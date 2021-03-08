use std::cmp::{min, max};
use crate::engine;
use crate::fontdata::{Glyph, GLYPH, LINE_HEIGHT};
use crate::cell_grid::{Point, coord_add, coord_subtract, coord_mul_components};
use crate::color_preset;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PopupType {
    Noise,
    Damage,
    GuardSpeech,
    Narration,
}

pub struct Popup {
    pub popup_type: PopupType,
    pub world_origin: Point, // world tile position
    pub msg: &'static str,
}

struct PopupPlaced {
    pub popup_type: PopupType,
    pub world_origin: Point, // world tile position
    pub msg: &'static str,
    pub size: Point, // 16x16 tiles
    pub offset: Point, // pixels, from top left to first character position
    pub pos: Point,
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
    
    pub fn guard_speech(&mut self, pos: Point, s: &'static str) {
        self.push(PopupType::GuardSpeech, pos, s);
        println!("{}", s);
    }

    pub fn damage(&mut self, pos: Point, s: &'static str) {
        self.push(PopupType::Damage, pos, s);
        println!("{}", s);
    }

    pub fn noise(&mut self, pos: Point, s: &'static str) {
        self.push(PopupType::Noise, pos, s);
        println!("{}", s);
    }

    pub fn draw(&self, screen_size_x: i32, screen_size_y: i32, view_scale: Point, view_offset: Point, focus: Point) {
        let view_min = (0, 0);
        let view_max = (screen_size_x, screen_size_y);
        let placed_popups = layout(view_min, view_max, focus, &self.popups);
        for p in &placed_popups {
			draw_popup(view_scale, view_offset, p);
        }
    }

    fn push(&mut self, popup_type: PopupType, pos: Point, s: &'static str) {
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
		PopupType::Damage => color_preset::LIGHT_RED,
		PopupType::GuardSpeech => color_preset::LIGHT_MAGENTA,
		PopupType::Narration => color_preset::WHITE,
	}
}

/*
fn fill_rect(window: &mut Window, font_image: &Image, view_scale: Point view_offset: Point, x_min: i32, y_min: i32, x_size: i32, y_size: i32, attr: u8, ch: u8)
{
	for x in 0..x_size {
		for y in 0..y_size {
			viewport.put_char(x + x_min, y + y_min, attr, ch);
		}
	}
}
*/

fn draw_popup(view_scale: Point, view_offset: Point, p: &PopupPlaced) {

	let has_box = p.popup_type != PopupType::Noise && p.popup_type != PopupType::Damage;

/*
	// Draw background and border.

	let mut text_x = p.pos.0;
	let mut text_y = p.pos.1 + p.size.1;

	if has_box {
		text_x += 1;
		text_y -= 1;

		let sx = p.size.0;
		let sy = p.size.1;
		let x0 = p.pos.0;
		let x1 = p.pos.0 + sx - 1;
		let y0 = p.pos.1;
		let y1 = p.pos.1 + sy - 1;

		viewport.put_char(x0, y0, edge_attr, 228);
		viewport.put_char(x1, y0, edge_attr, 229);
		viewport.put_char(x0, y1, edge_attr, 230);
		viewport.put_char(x1, y1, edge_attr, 231);

		let edge_attr: u8 = POPUP_EDGE_ATTR[p.popup_type];

		window.draw(
			&Rectangle::new(pos_px, image.area().size()),
			Blended(&image, color),
		);

		fill_rect(viewport, x0 + 1, y0, sx-2, 1, edge_attr, 226);
		fill_rect(viewport, x0 + 1, y1, sx-2, 1, edge_attr, 227);
		fill_rect(viewport, x0, y0 + 1, 1, sy-2, edge_attr, 224);
		fill_rect(viewport, x1, y0 + 1, 1, sy-2, edge_attr, 225);
		fill_rect(viewport, x0 + 1, y0 + 1, sx - 2, sy - 2, edge_attr, 0x20);
	}
*/

	// Draw the text.

//	let text_pos = viewport.to_screen(Point{ x: text_x, y: text_y });
	let text_pos = coord_add(view_offset, coord_mul_components(view_scale, p.pos));

	let x_start = text_pos.0;
	let y_start = text_pos.1 - (LINE_HEIGHT + p.offset.1) + if has_box {0} else {-16};

	if !has_box {
		puts_proportional(x_start + 2, y_start - 2, p.msg, color_preset::BLACK);
		puts_proportional(x_start + 1, y_start - 1, p.msg, color_preset::BLACK);
	}

	puts_proportional(x_start, y_start, p.msg, text_color(p.popup_type));
}

type Score = (i32, f32);

fn compute_score(
    view_min: Point,
    view_max: Point,
    origin: Point,
    dir: Point,
    pos: Point,
    size: Point
) -> Score {
    let mut box_min = pos;
    let mut box_max = coord_add(pos, size);
    let mut offscreen_area = 0;

    if box_min.0 < view_min.0 {
        offscreen_area += (view_min.0 - box_min.0) * (box_max.1 - box_min.1);
        box_min.0 = view_min.0;
    }
    if box_max.0 > view_max.0 {
        offscreen_area += (box_max.0 - view_max.0) * (box_max.1 - box_min.1);
        box_max.0 = view_max.0;
    }
    if box_min.1 < view_min.1 {
        offscreen_area += (view_min.1 - box_min.1) * (box_max.0 - box_min.0);
        box_min.1 = view_min.1;
    }
    if box_max.1 > view_max.1 {
        offscreen_area += (box_max.1 - view_max.1) * (box_max.0 - box_min.0);
        box_max.1 = view_max.1;
    }

    let dir2_x = (pos.0 + size.0) - 2 * origin.0;
    let dir2_y = (pos.1 + size.1) - 2 * origin.1;

    let dot = (dir2_x * dir.0 + dir2_y * dir.1) as f32 / ((dir2_x * dir2_x + dir2_y * dir2_y) as f32).sqrt();

    (offscreen_area, dot)
}

fn size_and_offset(p: &Popup) -> (Point, Point) {

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

    let size_internal = (
        (width + TILE_SCREEN_SIZE - 1) / TILE_SCREEN_SIZE,
        (height + TILE_SCREEN_SIZE - 1) / TILE_SCREEN_SIZE
    );

    let offset = (
        (TILE_SCREEN_SIZE * size_internal.0 - width) / 2 - x_min,
        (TILE_SCREEN_SIZE * size_internal.1 - height) / 2
    );

    // Non-noise text boxes have borders.

    let size = if p.popup_type == PopupType::Noise || p.popup_type == PopupType::Damage {
        size_internal
    } else {
        coord_add(size_internal, (2, 2))
    };

    (size, offset)
}

fn layout_single(view_min: Point, view_max: Point, focus: Point, p: &Popup) -> PopupPlaced {

    let (size, offset) = size_and_offset(p);

    let mut pos = (0, 0);

    if p.popup_type == PopupType::Narration {
        // Center narration.
        pos.0 = (view_min.0 + view_max.0 - size.0) / 2;
        pos.1 = (view_min.1 + view_max.1 - size.1) / 2;
    } else {
        // Search for a position that is on the opposite side of the source
        // from the focus, with a center position as close as possible to
        // the line between source and focus.

        let mut dir = coord_subtract(p.world_origin, focus);
        if dir.0 == 0 && dir.1 == 0 {
            dir.0 = 1;
        }

        let mut score_best: Score = (10000, -1000.0);

        // Generate positions along the top

        for x in 0..size.0 {
            let pos_trial = coord_add(p.world_origin, (-x, 2));
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }

        // Generate positions along the bottom

        for x in 0..size.0 {
            let pos_trial = coord_add(p.world_origin, (-x, -size.1));
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }

        // Generate positions along the sides

        for y in 0..size.1 {
            let pos_trial = coord_add(p.world_origin, (-size.0, -y));
            let score = compute_score(view_min, view_max, p.world_origin, dir, pos_trial, size);
            if score < score_best {
                score_best = score;
                pos = pos_trial;
            }
        }

        for y in 0..size.1 {
            let pos_trial = coord_add(p.world_origin, (1, -y));
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

fn layout(view_min: Point, view_max: Point, focus: Point, popups: &[Popup]) -> Vec<PopupPlaced> {
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
			engine::draw_tile(x + glyph.x_offset, y + glyph.y_offset, glyph.width, glyph.height, color, TEXTURE_INDEX, glyph.x, glyph.y);
            x += glyph.x_advance;
        }
    }

    x
}
