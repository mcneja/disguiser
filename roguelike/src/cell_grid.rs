use crate::color_preset;
use multiarray::Array2D;
use rand::Rng;
use std::cmp::max;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;

pub type Random = rand_pcg::Pcg32;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CellType {
    GroundNormal,
    GroundGrass,
    GroundWater,
    GroundMarble,
    GroundWood,
    GroundWoodCreaky,

    //  NSEW
    Wall0000,
    Wall0001,
    Wall0010,
    Wall0011,
    Wall0100,
    Wall0101,
    Wall0110,
    Wall0111,
    Wall1000,
    Wall1001,
    Wall1010,
    Wall1011,
    Wall1100,
    Wall1101,
    Wall1110,
    Wall1111,

    OneWayWindowE,
    OneWayWindowW,
    OneWayWindowN,
    OneWayWindowS,
    PortcullisNS,
    PortcullisEW,
    DoorNS,
    DoorEW,
}

pub const INVALID_REGION: usize = std::usize::MAX;
pub const INFINITE_COST: usize = std::usize::MAX;

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub cell_type: CellType,
    pub move_cost: usize,
    pub region: usize,
    pub blocks_player_sight: bool,
    pub blocks_sight: bool,
    pub blocks_sound: bool,
    pub hides_player: bool,
    pub lit: bool,
    pub seen: bool,
}

pub type CellGrid = Array2D<Cell>;
pub type Point = (i32, i32);

pub fn coord_add(coord0: Point, coord1: Point) -> Point {
    (coord0.0 + coord1.0, coord0.1 + coord1.1)
}

pub fn coord_subtract(coord0: Point, coord1: Point) -> Point {
    (coord0.0 - coord1.0, coord0.1 - coord1.1)
}

pub fn coord_negate(coord: Point) -> Point {
    (-coord.0, -coord.1)
}

pub fn coord_scale(coord: Point, scale: i32) -> Point {
    (coord.0 * scale, coord.1 * scale)
}

pub fn coord_dot(coord0: Point, coord1: Point) -> i32 {
    coord0.0 * coord1.0 + coord0.1 * coord1.1
}

pub fn coord_length_squared(coord: Point) -> i32 {
    coord.0*coord.0 + coord.1*coord.1
}

pub fn coord_mul_components(coord0: Point, coord1: Point) -> Point {
    (coord0.0 * coord1.0, coord0.1 * coord1.1)
}

pub struct Rect {
    pub pos_min: Point,
    pub pos_max: Point,
}

pub struct Map {
    pub cells: CellGrid,
    pub patrol_regions: Vec<Rect>,
    pub patrol_routes: Vec<(usize, usize)>,
    pub items: Vec<Item>,
    pub guards: Vec<Guard>,
    pub pos_start: Point,
    pub total_loot: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GuardMode
{
    Patrol,
    Look,
    Listen,
    ChaseVisibleTarget,
    MoveToLastSighting,
    MoveToLastSound,
    MoveToGuardShout,
}

pub struct Guard {
    pub pos: Point,
    pub dir: Point,
    pub mode: GuardMode,
    pub speaking: bool,
    pub has_moved: bool,
    pub heard_thief: bool,
    pub hearing_guard: bool,
    pub heard_guard: bool,
    pub heard_guard_pos: Point,

    // Chase
    pub goal: Point,
    pub mode_timeout: usize,

    // Patrol
    pub region_goal: usize,
    pub region_prev: usize,
}

pub struct Item {
    pub pos: Point,
    pub kind: ItemKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ItemKind {
    Chair,
    Table,
    Bush,
    Coin,
    DoorNS,
    DoorEW,
    PortcullisNS,
    PortcullisEW,
}

pub struct Player {
    pub pos: Point,
    pub dir: Point,
    pub max_health: usize,
    pub health: usize,
    pub gold: usize,

    pub noisy: bool, // did the player make noise last turn?
    pub damaged_last_turn: bool,
    pub finished_level: bool,

    pub turns_remaining_underwater: usize,

    pub see_all: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub glyph: u32,
    pub color: u32,
    pub blocks_player: bool,
    pub blocks_player_sight: bool,
    pub blocks_sight: bool,
    pub blocks_sound: bool,
    pub hides_player: bool,
    pub ignores_lighting: bool,
}

pub fn tile_def(tile_type: CellType) -> &'static Tile {
    match tile_type {
        CellType::GroundNormal     => &Tile { glyph: 128, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: false },
        CellType::GroundGrass      => &Tile { glyph: 132, color: color_preset::DARK_GREEN, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: false },
        CellType::GroundWater      => &Tile { glyph: 134, color: color_preset::LIGHT_BLUE, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: false },
        CellType::GroundMarble     => &Tile { glyph: 136, color: color_preset::DARK_CYAN, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: false },
        CellType::GroundWood       => &Tile { glyph: 138, color: color_preset::DARK_BROWN, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: false },
        CellType::GroundWoodCreaky => &Tile { glyph: 138, color: color_preset::DARK_BROWN, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: false },

                  //  NSEW
        CellType::Wall0000 => &Tile { glyph: 176, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: false, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0001 => &Tile { glyph: 177, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0010 => &Tile { glyph: 177, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0011 => &Tile { glyph: 177, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0100 => &Tile { glyph: 178, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0101 => &Tile { glyph: 179, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0110 => &Tile { glyph: 182, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall0111 => &Tile { glyph: 185, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1000 => &Tile { glyph: 178, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1001 => &Tile { glyph: 180, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1010 => &Tile { glyph: 181, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1011 => &Tile { glyph: 184, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1100 => &Tile { glyph: 178, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1101 => &Tile { glyph: 186, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1110 => &Tile { glyph: 183, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },
        CellType::Wall1111 => &Tile { glyph: 187, color: color_preset::LIGHT_GRAY, blocks_player: true, blocks_player_sight: true, blocks_sight: true, blocks_sound: true, hides_player: false, ignores_lighting: true },

        CellType::OneWayWindowE => &Tile { glyph: 196, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: true, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::OneWayWindowW => &Tile { glyph: 197, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: true, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::OneWayWindowN => &Tile { glyph: 198, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: true, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::OneWayWindowS => &Tile { glyph: 199, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: true, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::PortcullisNS  => &Tile { glyph: 128, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::PortcullisEW  => &Tile { glyph: 128, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::DoorNS        => &Tile { glyph: 189, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: true },
        CellType::DoorEW        => &Tile { glyph: 188, color: color_preset::LIGHT_GRAY, blocks_player: false, blocks_player_sight: false, blocks_sight: false, blocks_sound: false, hides_player: false, ignores_lighting: true },
    }
}

pub fn guard_move_cost_for_tile_type(tile_type: CellType) -> usize {
    match tile_type {
        CellType::GroundNormal     => 0,
        CellType::GroundGrass      => 0,
        CellType::GroundWater      => 4096,
        CellType::GroundMarble     => 0,
        CellType::GroundWood       => 0,
        CellType::GroundWoodCreaky => 0,
        CellType::Wall0000         => INFINITE_COST,
        CellType::Wall0001         => INFINITE_COST,
        CellType::Wall0010         => INFINITE_COST,
        CellType::Wall0011         => INFINITE_COST,
        CellType::Wall0100         => INFINITE_COST,
        CellType::Wall0101         => INFINITE_COST,
        CellType::Wall0110         => INFINITE_COST,
        CellType::Wall0111         => INFINITE_COST,
        CellType::Wall1000         => INFINITE_COST,
        CellType::Wall1001         => INFINITE_COST,
        CellType::Wall1010         => INFINITE_COST,
        CellType::Wall1011         => INFINITE_COST,
        CellType::Wall1100         => INFINITE_COST,
        CellType::Wall1101         => INFINITE_COST,
        CellType::Wall1110         => INFINITE_COST,
        CellType::Wall1111         => INFINITE_COST,
        CellType::OneWayWindowE    => INFINITE_COST,
        CellType::OneWayWindowW    => INFINITE_COST,
        CellType::OneWayWindowN    => INFINITE_COST,
        CellType::OneWayWindowS    => INFINITE_COST,
        CellType::PortcullisNS     => 0,
        CellType::PortcullisEW     => 0,
        CellType::DoorNS           => 0,
        CellType::DoorEW           => 0,
    }
}

pub fn guard_move_cost_for_item_kind(kind: ItemKind) -> usize {
    match kind {
        ItemKind::Chair => 4,
        ItemKind::Table => 10,
        ItemKind::Bush => 10,
        ItemKind::Coin => 0,
        ItemKind::DoorNS => 0,
        ItemKind::DoorEW => 0,
        ItemKind::PortcullisNS => 0,
        ItemKind::PortcullisEW => 0,
    }
}

pub fn make_player(pos: &Point) -> Player {
    let health = 5;
    Player {
        pos: *pos,
        dir: (0, 0),
        max_health: health,
        health: health,
        gold: 0,
        noisy: false,
        damaged_last_turn: false,
        finished_level: false,
        turns_remaining_underwater: 0,
        see_all: false,
    }
}

impl Player {
    pub fn apply_damage(&mut self, d: usize) {
        self.health = max(0, self.health - d);
        self.damaged_last_turn = true;
    }

    pub fn hidden(&self, map: &Map) -> bool {
        for guard in &map.guards {
            if guard.mode == GuardMode::ChaseVisibleTarget {
                return false;
            }
        }

        if map.hides_player(self.pos.0, self.pos.1) {
            return true;
        }

        let cell_type = map.cells[[self.pos.0 as usize, self.pos.1 as usize]].cell_type;

        if cell_type == CellType::GroundWater && self.turns_remaining_underwater > 0 {
            return true;
        }

        false
    }
}

const ADJACENT_MOVES: [(usize, Point); 8] = [
    (2, (1, 0)),
    (2, (-1, 0)),
    (2, (0, 1)),
    (2, (0, -1)),
    (3, (-1, 1)),
    (3, (1, -1)),
    (3, (-1, 1)),
    (3, (1, 1)),
];

const SOUND_NEIGHBORS: [Point; 4] = [
    (-1, 0),
    (1, 0),
    (0, -1),
    (0, 1),
];

struct PortalInfo {
    // offset of left corner of portal relative to lower-left corner of cell:
    lx: i32,
    ly: i32,
    // offset of right corner of portal relative to lower-left-corner of cell:
    rx: i32,
    ry: i32,
    // offset of neighboring cell relative to this cell's coordinates:
    nx: i32,
    ny: i32,
}

const PORTAL: [PortalInfo; 4] = [
    // lx, ly   rx, ry   nx, ny
    PortalInfo { lx: -1, ly: -1, rx: -1, ry:  1, nx: -1, ny:  0 },
    PortalInfo { lx: -1, ly:  1, rx:  1, ry:  1, nx:  0, ny:  1 },
    PortalInfo { lx:  1, ly:  1, rx:  1, ry: -1, nx:  1, ny:  0 },
    PortalInfo { lx:  1, ly: -1, rx: -1, ry: -1, nx:  0, ny: -1 },
];

fn a_right_of_b(ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    ax * by > ay * bx
}

fn allowed_direction(tile_type: CellType, dx: i32, dy: i32) -> bool {
    match tile_type {
        CellType::OneWayWindowE => dx > 0,
        CellType::OneWayWindowW => dx < 0,
        CellType::OneWayWindowN => dy > 0,
        CellType::OneWayWindowS => dy < 0,
        _ => true
    }
}

impl Map {

pub fn collect_loot_at(&mut self, pos: Point) -> usize {
    let mut gold = 0;
    self.items.retain(|item| if item.kind == ItemKind::Coin && item.pos == pos {gold += 1; false} else {true});
    gold
}

pub fn all_seen(&self) -> bool {
    // There's got to be a better way to iterate over all the cells...
    for x in 0..self.cells.extents()[0] {
        for y in 0..self.cells.extents()[1] {
            if !self.cells[[x, y]].seen {
                return false;
            }
        }
    }
    true
}

pub fn percent_seen(&self) -> usize {
    let mut num_seen: usize = 0;
    for x in 0..self.cells.extents()[0] {
        for y in 0..self.cells.extents()[1] {
            if self.cells[[x, y]].seen {
                num_seen += 1;
            }
        }
    }

    let num_to_see: usize = self.cells.extents()[0] * self.cells.extents()[1];
    (num_seen * 100) / num_to_see
}

pub fn mark_all_seen(&mut self) {
    for x in 0..self.cells.extents()[0] {
        for y in 0..self.cells.extents()[1] {
            self.cells[[x, y]].seen = true;
        }
    }
}

pub fn mark_all_unseen(&mut self) {
    for x in 0..self.cells.extents()[0] {
        for y in 0..self.cells.extents()[1] {
            self.cells[[x, y]].seen = false;
        }
    }
}

pub fn recompute_visibility(&mut self, pos_viewer: Point) {
    for portal in &PORTAL {
        self.compute_visibility
        (
            pos_viewer.0, pos_viewer.1,
            pos_viewer.0, pos_viewer.1,
            portal.lx, portal.ly,
            portal.rx, portal.ry
        );
    }
}

fn compute_visibility(
    &mut self,
    // Viewer map coordinates:
    viewer_x: i32,
    viewer_y: i32,
    // Target cell map coordinates:
    target_x: i32,
    target_y: i32,
    // Left edge of current view frustum (relative to viewer):
    ldx: i32,
    ldy: i32,
    // Right edge of current view frustum (relative to viewer):
    rdx: i32,
    rdy: i32
) {
    // End recursion if the target cell is out of bounds.
    if target_x < 0 || target_y < 0 || target_x as usize >= self.cells.extents()[0] || target_y as usize >= self.cells.extents()[1] {
        return;
    }

    // End recursion if the target square is too far away.
    let (dx, dy) = (2 * (target_x - viewer_x), 2 * (target_y - viewer_y));

    if dx*dx + dy*dy > 1600 {
        return;
    }

    // End recursion if the incoming direction is not allowed by the current cell type.
    if !allowed_direction(self.cells[[target_x as usize, target_y as usize]].cell_type, dx, dy) {
        return;
    }

    // This square is visible.
    self.cells[[target_x as usize, target_y as usize]].seen = true;

    // End recursion if the target square occludes the view.
    if self.blocks_player_sight(target_x, target_y) {
        return;
    }

    // Mark diagonally-adjacent squares as visible if their corners are visible
    for x in 0..2 {
        for y in 0..2 {
            let nx = target_x + 2*x - 1;
            let ny = target_y + 2*y - 1;
            let cdx = dx + 2*x - 1;
            let cdy = dy + 2*y - 1;
            
            if nx >= 0 &&
               ny >= 0 &&
               (nx as usize) < self.cells.extents()[0] &&
               (ny as usize) < self.cells.extents()[1] &&
               !a_right_of_b(ldx, ldy, cdx, cdy) &&
               !a_right_of_b(cdx, cdy, rdx, rdy) {
                self.cells[[nx as usize, ny as usize]].seen = true;
            }
        }
    }

    // Clip portals to adjacent squares and recurse through the visible portions
    for portal in &PORTAL {
        // Relative positions of the portal's left and right endpoints:
        let (pldx, pldy) = (dx + portal.lx, dy + portal.ly);
        let (prdx, prdy) = (dx + portal.rx, dy + portal.ry);

        // Clip portal against current view frustum:
        let (cldx, cldy) = if a_right_of_b(ldx, ldy, pldx, pldy) {
            (ldx, ldy)
        } else {
            (pldx, pldy)
        };
        let (crdx, crdy) = if a_right_of_b(rdx, rdy, prdx, prdy) {
            (prdx, prdy)
        } else {
            (rdx, rdy)
        };

        // If we can see through the clipped portal, recurse through it.
        if a_right_of_b(crdx, crdy, cldx, cldy) {
            self.compute_visibility
            (
                viewer_x, viewer_y,
                target_x + portal.nx, target_y + portal.ny,
                cldx, cldy,
                crdx, crdy
            );
        }
    }
}

pub fn all_loot_collected(&self) -> bool {
    for item in &self.items {
        if item.kind == ItemKind::Coin {
            return false;
        }
    }
    true
}

pub fn random_neighbor_region(&self, random: &mut Random, region: usize, region_exclude: usize) -> usize {
    let mut neighbors: Vec<usize> = Vec::with_capacity(8);

    for (region0, region1) in &self.patrol_routes {
        if *region0 == region && *region1 != region_exclude {
            neighbors.push(*region1);
        } else if *region1 == region && *region0 != region_exclude {
            neighbors.push(*region0);
        }
    }

    if neighbors.is_empty() {
        return region;
    }

    return neighbors[random.gen_range(0..neighbors.len())];
}

fn guard_cell_cost(&self, x: usize, y: usize) -> usize {
    self.cells[[x, y]].move_cost
}

pub fn guard_move_cost(&self, pos_old: Point, pos_new: Point) -> usize {
    let cost = self.guard_cell_cost(pos_new.0 as usize, pos_new.1 as usize);

    if cost == INFINITE_COST {
        return cost;
    }

    // Guards are not allowed to move diagonally around corners.

    if pos_old.0 != pos_new.0 &&
        pos_old.1 != pos_new.1 &&
        (self.guard_cell_cost(pos_old.0 as usize, pos_new.1 as usize) == INFINITE_COST ||
        self.guard_cell_cost(pos_new.0 as usize, pos_old.1 as usize) == INFINITE_COST) {
        return INFINITE_COST;
    }

    cost
}

pub fn pos_blocked_by_guard(&self, pos: Point) -> bool {
    for guard in &self.guards {
        if guard.pos == pos {
            return true;
        }
    }

    false
}

pub fn closest_region(&self, pos: &Point) -> usize {

    #[derive(Copy, Clone, Eq, PartialEq)]
    struct State {
        dist: usize,
        pos: Point,
    }

    impl Ord for State {
        fn cmp(&self, other: &State) -> Ordering {
            other.dist.cmp(&self.dist)
        }
    }

    impl PartialOrd for State {
        fn partial_cmp(&self, other: &State) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut heap = BinaryHeap::with_capacity(self.cells.extents()[0] * self.cells.extents()[1]);
    let mut dist_field: Array2D<usize> = Array2D::new([self.cells.extents()[0], self.cells.extents()[1]], INFINITE_COST);

    heap.push(State{dist: 0, pos: *pos});

    let size_x = self.cells.extents()[0] as i32;
    let size_y = self.cells.extents()[1] as i32;

    while let Some(State {dist, pos}) = heap.pop() {
        let p = [pos.0 as usize, pos.1 as usize];

        if self.cells[p].region != INVALID_REGION {
            return self.cells[p].region;
        }

        if dist >= dist_field[p] {
            continue;
        }

        dist_field[p] = dist;

        for (move_dir_cost, dir) in &ADJACENT_MOVES {
            let pos_new = coord_add(pos, *dir);
            if pos_new.0 < 0 || pos_new.1 < 0 || pos_new.0 >= size_x || pos_new.1 >= size_y {
                continue;
            }

            let move_cost = self.guard_move_cost(pos, pos_new);
            if move_cost == INFINITE_COST {
                continue;
            }

            let dist_new = dist + move_cost + move_dir_cost;

            if dist_new < dist_field[[pos_new.0 as usize, pos_new.1 as usize]] {
                heap.push(State{dist: dist_new, pos: pos_new});
            }
        }
    }

    INVALID_REGION
}

pub fn compute_distances_to_region(&self, i_region_goal: usize) -> Array2D<usize> {
    assert!(i_region_goal < self.patrol_regions.len());

    let region = &self.patrol_regions[i_region_goal];

    // Fill the priority queue with all of the region's locations.

    let mut goal = Vec::with_capacity(((region.pos_max.0 - region.pos_min.0) * (region.pos_max.1 - region.pos_min.1)) as usize);

    for x in region.pos_min.0 .. region.pos_max.0 {
        for y in region.pos_min.1 .. region.pos_max.1 {
            let p = (x, y);
            goal.push((self.guard_cell_cost(x as usize, y as usize), p));
        }
    }

    self.compute_distance_field(&goal)
}

pub fn compute_distances_to_position(&self, pos_goal: Point) -> Array2D<usize> {
    assert!(pos_goal.0 >= 0);
    assert!(pos_goal.1 >= 0);
    assert!(pos_goal.0 < self.cells.extents()[0] as i32);
    assert!(pos_goal.1 < self.cells.extents()[1] as i32);

    self.compute_distance_field(&[(0, pos_goal)])
}

pub fn compute_distance_field(&self, initial_distances: &[(usize, Point)]) -> Array2D<usize> {

    #[derive(Copy, Clone, Eq, PartialEq)]
    struct State {
        dist: usize,
        pos: Point,
    }

    impl Ord for State {
        fn cmp(&self, other: &State) -> Ordering {
            other.dist.cmp(&self.dist)
        }
    }

    impl PartialOrd for State {
        fn partial_cmp(&self, other: &State) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut heap = BinaryHeap::with_capacity(self.cells.extents()[0] * self.cells.extents()[1]);
    let mut dist_field: Array2D<usize> = Array2D::new([self.cells.extents()[0], self.cells.extents()[1]], INFINITE_COST);

    let size_x = self.cells.extents()[0] as i32;
    let size_y = self.cells.extents()[1] as i32;

    for (dist, pos) in initial_distances {
        heap.push(State{dist: *dist, pos: *pos});
    }

    while let Some(State {dist, pos}) = heap.pop() {
        let p = [pos.0 as usize, pos.1 as usize];
        if dist >= dist_field[p] {
            continue;
        }

        dist_field[p] = dist;

        for (move_dir_cost, dir) in &ADJACENT_MOVES {
            let pos_new = coord_add(pos, *dir);
            if pos_new.0 < 0 || pos_new.1 < 0 || pos_new.0 >= size_x || pos_new.1 >= size_y {
                continue;
            }

            let move_cost = self.guard_move_cost(pos, pos_new);
            if move_cost == INFINITE_COST {
                continue;
            }

            let dist_new = dist + move_cost + move_dir_cost;

            if dist_new < dist_field[[pos_new.0 as usize, pos_new.1 as usize]] {
                heap.push(State{dist: dist_new, pos: pos_new});
            }
        }
    }

    dist_field
}

pub fn blocks_sight(&self, x: i32, y: i32) -> bool {
    self.cells[[x as usize, y as usize]].blocks_sight
}

pub fn blocks_player_sight(&self, x: i32, y: i32) -> bool {
    self.cells[[x as usize, y as usize]].blocks_player_sight
}

pub fn hides_player(&self, x: i32, y: i32) -> bool {
    self.cells[[x as usize, y as usize]].hides_player
}

pub fn find_guards_in_earshot(&mut self, emitter_pos: Point, radius: i32) -> Vec<&mut Guard> {
    let mut visited: Array2D<bool> = Array2D::new([self.cells.extents()[0], self.cells.extents()[1]], false);

    // Flood-fill from the emitter position.

    let mut points: VecDeque<Point> = VecDeque::new();
    points.push_back(emitter_pos);
    visited[[emitter_pos.0 as usize, emitter_pos.1 as usize]] = true;

    while let Some(pos) = points.pop_front() {
        for dir in &SOUND_NEIGHBORS {
            let new_pos = coord_add(pos, *dir);

            // Skip positions that are off the map.

            if new_pos.0 < 0 || new_pos.0 >= self.cells.extents()[0] as i32 ||
                new_pos.1 < 0 || new_pos.1 >= self.cells.extents()[1] as i32 {
                continue;
            }

            // Skip neighbors that have already been visited.

            if visited[[new_pos.0 as usize, new_pos.1 as usize]] {
                continue;
            }

            // Skip neighbors that are outside of the hearing radius.

            let d = coord_subtract(new_pos, emitter_pos);
            let d2 = coord_length_squared(d);
            if d2 >= radius {
                continue;
            }

            // Skip neighbors that don't transmit sound

            if self.cells[[new_pos.0 as usize, new_pos.1 as usize]].blocks_sound {
                continue;
            }

            visited[[new_pos.0 as usize, new_pos.1 as usize]] = true;
            points.push_back(new_pos);
        }
    }

    // Return guards that are on marked squares.

    let mut guards = Vec::with_capacity(self.guards.len());

    for guard in &mut self.guards {
        if visited[[guard.pos.0 as usize, guard.pos.1 as usize]] {
            guards.push(guard);
        }
    }

    guards
}

}
