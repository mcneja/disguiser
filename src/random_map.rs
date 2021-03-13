use crate::cell_grid;
use crate::cell_grid::{Cell, CellGrid, CellType, INVALID_REGION, Item, ItemKind, Map, PatrolRegion, Random, Rect};
use crate::coord::Coord;
use crate::guard;

use rand::prelude::{Rng, SliceRandom};
use std::cmp::{Ordering, min, max};
use std::mem::swap;
use multiarray::Array2D;

const OUTER_BORDER: i32 = 3;

const ROOM_SIZE_X: i32 = 5;
const ROOM_SIZE_Y: i32 = 5;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RoomType
{
    Exterior,
    PublicCourtyard,
    PublicRoom,
    PrivateCourtyard,
    PrivateRoom,
}

struct Room
{
    pub room_type: RoomType,
    pub group: usize,
    pub depth: usize,
    pub dead_end: bool,
    pub patroller: Option<guard::GuardKind>,
    pub pos_min: Coord,
    pub pos_max: Coord,
    pub edges: Vec<usize>,
}

struct Adjacency
{
    pub origin: Coord,
    pub dir: Coord,
    pub length: i32,
    pub room_left: usize,
    pub room_right: usize,
    pub next_matching: usize,
    pub door: bool,
}

pub fn generate_map(random: &mut Random, level: usize) -> Map {

    for _ in 0..100 {
        let map = generate_siheyuan(random, level);
        if !map.patrol_regions.is_empty() {
            return map;
        }
    }

    generate_siheyuan(random, level)
}

fn generate_siheyuan(random: &mut Random, level: usize) -> Map {
    let mut size_x: i32 = 0;
    for _ in 0..min(3, level) {
        size_x += random.gen_range(0..2);
    }
    size_x *= 2;
    size_x += 3;

    let mut size_y: i32;
    if level == 0 {
        size_y = 2;
    } else {
        size_y = 3;
        for _ in 0..min(4, level - 1) {
            size_y += random.gen_range(0..2);
        }
    }

    let mirror_x = true;
    let mirror_y = false;

    let inside: Array2D<bool> = make_siheyuan_room_grid(size_x as usize, size_y as usize, random);

    // Compute wall offsets.

    let (offset_x, offset_y) = offset_walls(mirror_x, mirror_y, &inside, random);

    // Convert the room descriptions to walls.

    let mut cells = plot_walls(&inside, &offset_x, &offset_y);

    // Fix up walls.

    fixup_walls(&mut cells);

    // Create exits connecting rooms.

    let mut map = Map {
        cells: cells,
        items: Vec::new(),
        patrol_regions: Vec::new(),
        patrol_routes: Vec::new(),
        guards: Vec::new(),
        pos_start: Coord(0, 0),
        total_loot: 0,
    };

    let (rooms, adjacencies, pos_start) = create_exits(
        random,
        level,
        mirror_x,
        mirror_y,
        &inside,
        &offset_x,
        &offset_y,
        &mut map);

    map.pos_start = pos_start;

    // Place outfits.

    if level > 1 {
        place_outfits(random, &rooms, &mut map);
    }

    // Place loot.

    place_loot(random, &rooms, &adjacencies, &mut map);

    // Place exterior junk.

    place_exterior_bushes(random, &mut map);
    place_front_pillars(&mut map);

    // Place guards.

//  init_pathing(map);

    if level > 0 {
        place_guards_by_type(random, level, &rooms, &mut map, guard::GuardKind::Inner);
        place_guards_by_type(random, level, &rooms, &mut map, guard::GuardKind::Outer);
    }

    mark_exterior_as_seen(&mut map);

    cache_cell_info(&mut map);

    map.total_loot = map.items.iter().filter(|&item| item.kind == ItemKind::Coin).count();

    map
}

fn make_siheyuan_room_grid(size_x: usize, size_y: usize, random: &mut Random) -> Array2D<bool> {
    let mut inside = Array2D::new([size_x, size_y], true);

    let half_x = (size_x + 1) / 2;

    for _ in 0..(size_y * half_x) / 4 {
        let x = random.gen_range(0..half_x);
        let y = random.gen_range(0..size_y);
        inside[[x, y]] = false;
    }

    for y in 0..size_y {
        for x in half_x..size_x {
            inside[[x, y]] = inside[[(size_x - 1) - x, y]];
        }
    }

    inside
}

fn offset_walls(mirror_x: bool, mirror_y: bool, inside: &Array2D<bool>, random: &mut Random) -> (Array2D<i32>, Array2D<i32>) {
    let rooms_x = inside.extents()[0];
    let rooms_y = inside.extents()[1];

    let mut offset_x = Array2D::new([rooms_x + 1, rooms_y], 0);
    let mut offset_y = Array2D::new([rooms_x, rooms_y + 1], 0);

    {
        let i = random.gen_range(0..3) - 1;
        for y in 0..rooms_y {
            offset_x[[0, y]] = i;
        }
    }

    {
        let i = random.gen_range(0..3) - 1;
        for y in 0..rooms_y {
            offset_x[[rooms_x, y]] = i;
        }
    }

    {
        let i = random.gen_range(0..3) - 1;
        for x in 0..rooms_x {
            offset_y[[x, 0]] = i;
        }
    }

    {
        let i = random.gen_range(0..3) - 1;
        for x in 0..rooms_x {
            offset_y[[x, rooms_y]] = i;
        }
    }

    for x in 1..rooms_x {
        for y in 0..rooms_y {
            offset_x[[x, y]] = random.gen_range(0..3) - 1;
        }
    }

    for x in 0..rooms_x {
        for y in 1..rooms_y {
            offset_y[[x, y]] = random.gen_range(0..3) - 1;
        }
    }

    for x in 1..rooms_x {
        for y in 1..rooms_y {
            if random.gen() {
                offset_x[[x, y]] = offset_x[[x, y-1]];
            } else {
                offset_y[[x, y]] = offset_y[[x-1, y]];
            }
        }
    }

    if mirror_x {
        if (rooms_x & 1) == 0 {
            let x_mid = rooms_x / 2;
            for y in 0..rooms_y {
                offset_x[[x_mid, y]] = 0;
            }
        }

        for x in 0..(rooms_x + 1) / 2 {
            for y in 0..rooms_y {
                offset_x[[rooms_x - x, y]] = 1 - offset_x[[x, y]];
            }
        }

        for x in 0..rooms_x / 2 {
            for y in 0..rooms_y + 1 {
                offset_y[[(rooms_x - 1) - x, y]] = offset_y[[x, y]];
            }
        }
    }

    if mirror_y {
        if (rooms_y & 1) == 0 {
            let y_mid = rooms_y / 2;
            for x in 0..rooms_x {
                offset_y[[x, y_mid]] = 0;
            }
        }

        for y in 0..(rooms_y + 1) / 2 {
            for x in 0..rooms_x {
                offset_y[[x, rooms_y - y]] = 1 - offset_y[[x, y]];
            }
        }

        for y in 0..rooms_y / 2 {
            for x in 0..rooms_x + 1 {
                offset_x[[x, (rooms_y - 1) - y]] = offset_x[[x, y]];
            }
        }
    }

    let mut room_offset_x = std::i32::MIN;
    let mut room_offset_y = std::i32::MIN;

    for y in 0..rooms_y {
        room_offset_x = max(room_offset_x, -offset_x[[0, y]]);
    }

    for x in 0..rooms_x {
        room_offset_y = max(room_offset_y, -offset_y[[x, 0]]);
    }

    room_offset_x += OUTER_BORDER;
    room_offset_y += OUTER_BORDER;

    for x in 0..rooms_x + 1 {
        for y in 0..rooms_y {
            offset_x[[x, y]] += room_offset_x + (x as i32) * ROOM_SIZE_X;
        }
    }

    for x in 0..rooms_x {
        for y in 0..rooms_y + 1 {
            offset_y[[x, y]] += room_offset_y + (y as i32) * ROOM_SIZE_Y;
        }
    }

    (offset_x, offset_y)
}

fn plot_walls(inside: &Array2D<bool>, offset_x: &Array2D<i32>, offset_y: &Array2D<i32>) -> CellGrid {
    let cx = inside.extents()[0];
    let cy = inside.extents()[1];

    let mut map_x = 0;
    let mut map_y = 0;

    for y in 0..cy {
        map_x = max(map_x, offset_x[[cx, y]]);
    }

    for x in 0..cx {
        map_y = max(map_y, offset_y[[x, cy]]);
    }

    map_x += OUTER_BORDER + 1;
    map_y += OUTER_BORDER + 1;

    let default_cell = Cell {
        cell_type: CellType::GroundNormal,
        move_cost: 0,
        region: INVALID_REGION,
        blocks_player_sight: false,
        blocks_sight: false,
        blocks_sound: false,
        hides_player: false,
        lit: false,
        seen: false,
        inner: false,
    };
    let mut map = CellGrid::new([map_x as usize, map_y as usize], default_cell);

    // Super hacky: put down grass under all the rooms to plug holes, and light the interior.

    for rx in 0..cx {
        for ry in 0..cy {
            let x0 = offset_x[[rx, ry]];
            let x1 = offset_x[[rx + 1, ry]] + 1;
            let y0 = offset_y[[rx, ry]];
            let y1 = offset_y[[rx, ry + 1]] + 1;

            for x in x0..x1 {
                for y in y0..y1 {
                    let cell = &mut map[[x as usize, y as usize]];
                    cell.cell_type = CellType::GroundGrass;
                    cell.lit = true;
                }
            }
        }
    }

    // Draw walls. Really this should be done in createExits, where the
    //  walls are getting decorated with doors and windows.

    for rx in 0..cx {
        for ry in 0..cy {
            let indoors = inside[[rx, ry]];

            let x0 = offset_x[[rx, ry]];
            let x1 = offset_x[[rx + 1, ry]];
            let y0 = offset_y[[rx, ry]];
            let y1 = offset_y[[rx, ry + 1]];

            if rx == 0 || indoors {
                plot_ns_wall(&mut map, x0, y0, y1);
            }

            if rx == cx - 1 || indoors {
                plot_ns_wall(&mut map, x1, y0, y1);
            }

            if ry == 0 || indoors {
                plot_ew_wall(&mut map, x0, y0, x1);
            }

            if ry == cy - 1 || indoors {
                plot_ew_wall(&mut map, x0, y1, x1);
            }
        }
    }

    map
}

fn plot_ns_wall(map: &mut CellGrid, x0: i32, y0: i32, y1: i32) {
    for y in y0..=y1 {
        map[[x0 as usize, y as usize]].cell_type = CellType::Wall0000;
    }
}

fn plot_ew_wall(map: &mut CellGrid, x0: i32, y0: i32, x1: i32) {
    for x in x0..=x1 {
        map[[x as usize, y0 as usize]].cell_type = CellType::Wall0000;
    }
}

fn fixup_walls(map: &mut CellGrid) {
    for x in 0..map.extents()[0] {
        for y in 0..map.extents()[1] {
            if is_wall(map[[x, y]].cell_type) {
                map[[x, y]].cell_type = wall_type_from_neighbors(neighboring_walls(&map, x, y));
            }
        }
    }
}

fn wall_type_from_neighbors(neighbors: u32) -> CellType {
    match neighbors {
        0  => CellType::Wall0000,
        1  => CellType::Wall0001,
        2  => CellType::Wall0010,
        3  => CellType::Wall0011,
        4  => CellType::Wall0100,
        5  => CellType::Wall0101,
        6  => CellType::Wall0110,
        7  => CellType::Wall0111,
        8  => CellType::Wall1000,
        9  => CellType::Wall1001,
        10 => CellType::Wall1010,
        11 => CellType::Wall1011,
        12 => CellType::Wall1100,
        13 => CellType::Wall1101,
        14 => CellType::Wall1110,
        15 => CellType::Wall1111,
        _  => CellType::Wall0000,
    }
}

fn is_wall(cell_type: CellType) -> bool {
    match cell_type {
        CellType::GroundNormal     => false,
        CellType::GroundGrass      => false,
        CellType::GroundWater      => false,
        CellType::GroundMarble     => false,
        CellType::GroundWood       => false,
        CellType::GroundWoodCreaky => false,

                  //  NSEW
        CellType::Wall0000 => true,
        CellType::Wall0001 => true,
        CellType::Wall0010 => true,
        CellType::Wall0011 => true,
        CellType::Wall0100 => true,
        CellType::Wall0101 => true,
        CellType::Wall0110 => true,
        CellType::Wall0111 => true,
        CellType::Wall1000 => true,
        CellType::Wall1001 => true,
        CellType::Wall1010 => true,
        CellType::Wall1011 => true,
        CellType::Wall1100 => true,
        CellType::Wall1101 => true,
        CellType::Wall1110 => true,
        CellType::Wall1111 => true,

        CellType::OneWayWindowE => true,
        CellType::OneWayWindowW => true,
        CellType::OneWayWindowN => true,
        CellType::OneWayWindowS => true,
        CellType::PortcullisNS  => true,
        CellType::PortcullisEW  => true,
        CellType::DoorNS        => true,
        CellType::DoorEW        => true,
    }
}

fn neighboring_walls(map: &CellGrid, x: usize, y: usize) -> u32 {
    let size_x = map.extents()[0];
    let size_y = map.extents()[1];
    let mut wall_bits = 0;

    if y < size_y-1 && is_wall(map[[x, y+1]].cell_type) {
        wall_bits |= 8;
    }
    if y > 0 && is_wall(map[[x, y-1]].cell_type) {
        wall_bits |= 4;
    }
    if x < size_x-1 && is_wall(map[[x+1, y]].cell_type) {
        wall_bits |= 2;
    }
    if x > 0 && is_wall(map[[x-1, y]].cell_type) {
        wall_bits |= 1;
    }

    wall_bits
}

fn create_exits(
    random: &mut Random,
    level: usize,
    mirror_x: bool,
    mirror_y: bool,
    inside: &Array2D<bool>,
    offset_x: &Array2D<i32>,
    offset_y: &Array2D<i32>,
    map: &mut Map
) -> (Vec<Room>, Vec<Adjacency>, Coord) {
    // Make a set of rooms.

    let rooms_x: usize = inside.extents()[0];
    let rooms_y: usize = inside.extents()[1];

    let mut room_index: Array2D<usize> = Array2D::new([rooms_x, rooms_y], 0);
    let mut rooms: Vec<Room> = Vec::new();

    // This room represents the area surrounding the map.

    rooms.push(
        Room {
            room_type: RoomType::Exterior,
            group: 0,
            depth: 0,
            dead_end: true,
            patroller: None,
            pos_min: Coord(0, 0), // not meaningful for this room
            pos_max: Coord(0, 0), // not meaningful for this room
            edges: Vec::new(),
        }
    );

    for rx in 0..rooms_x {
        for ry in 0..rooms_y {
            let group_index = rooms.len();

            room_index[[rx, ry]] = group_index;

            rooms.push(
                Room {
                    room_type: if inside[[rx, ry]] {RoomType::PublicRoom} else {RoomType::PublicCourtyard},
                    group: group_index,
                    depth: 0,
                    dead_end: false,
                    patroller: None,
                    pos_min: Coord(offset_x[[rx, ry]] + 1, offset_y[[rx, ry]] + 1),
                    pos_max: Coord(offset_x[[rx + 1, ry]], offset_y[[rx, ry + 1]]),
                    edges: Vec::new(),
                }
            );
        }
    }

    // Compute a list of room adjacencies.

    let mut adjacencies = compute_adjacencies(mirror_x, mirror_y, &inside, &offset_x, &offset_y, &room_index);
    store_adjacencies_in_rooms(&adjacencies, &mut rooms);

    // Connect rooms together.

    let pos_start = connect_rooms(random, &mut rooms, &mut adjacencies);

    // Assign types to the rooms.

    assign_room_types(&room_index, &adjacencies, &mut rooms);

    // Generate pathing information.

    generate_patrol_routes(map, &mut rooms, &adjacencies);

    // Render doors and windows.

    render_walls(random, &rooms, &adjacencies, map);

    // Render floors.

    render_rooms(level, &rooms, map, random);

    (rooms, adjacencies, pos_start)
}

fn compute_adjacencies
(
    mirror_x: bool,
    mirror_y: bool,
    inside: &Array2D<bool>,
    offset_x: &Array2D<i32>,
    offset_y: &Array2D<i32>,
    room_index: &Array2D<usize>
) -> Vec<Adjacency> {

    let rooms_x = inside.extents()[0];
    let rooms_y = inside.extents()[1];

    let mut adjacencies: Vec<Adjacency> = Vec::new();

    {
        let mut adjacency_rows: Vec<Vec<usize>> = Vec::with_capacity(rooms_y + 1);

        {
            let mut adjacency_row: Vec<usize> = Vec::with_capacity(rooms_x);

            let ry = 0;

            for rx in 0..rooms_x {
                let x0 = offset_x[[rx, ry]];
                let x1 = offset_x[[rx+1, ry]];
                let y = offset_y[[rx, ry]];

                let i = adjacencies.len();
                adjacency_row.push(i);

                adjacencies.push(
                    Adjacency {
                        origin: Coord(x0 + 1, y),
                        dir: Coord(1, 0),
                        length: x1 - (x0 + 1),
                        room_left: room_index[[rx, ry]],
                        room_right: 0,
                        next_matching: i,
                        door: false,
                    }
                );
            }

            adjacency_rows.push(adjacency_row);
        }

        for ry in 1..rooms_y {
            let mut adjacency_row: Vec<usize> = Vec::with_capacity(3 * rooms_x);

            for rx in 0..rooms_x {
                let x0_upper = offset_x[[rx, ry]];
                let x0_lower = offset_x[[rx, ry-1]];
                let x1_upper = offset_x[[rx+1, ry]];
                let x1_lower = offset_x[[rx+1, ry-1]];
                let x0 = max(x0_lower, x0_upper);
                let x1 = min(x1_lower, x1_upper);
                let y = offset_y[[rx, ry]];

                if rx > 0 && x0_lower - x0_upper > 1 {
                    let i = adjacencies.len();
                    adjacency_row.push(i);

                    adjacencies.push(
                        Adjacency {
                            origin: Coord(x0_upper + 1, y),
                            dir: Coord(1, 0),
                            length: x0_lower - (x0_upper + 1),
                            room_left: room_index[[rx, ry]],
                            room_right: room_index[[rx - 1, ry - 1]],
                            next_matching: i,
                            door: false,
                        }
                    );
                }

                if x1 - x0 > 1 {
                    let i = adjacencies.len();
                    adjacency_row.push(i);

                    adjacencies.push(
                        Adjacency {
                            origin: Coord(x0 + 1, y),
                            dir: Coord(1, 0),
                            length: x1 - (x0 + 1),
                            room_left: room_index[[rx, ry]],
                            room_right: room_index[[rx, ry - 1]],
                            next_matching: i,
                            door: false,
                        }
                    );
                }

                if rx + 1 < rooms_x && x1_upper - x1_lower > 1 {
                    let i = adjacencies.len();
                    adjacency_row.push(i);

                    adjacencies.push(
                        Adjacency {
                            origin: Coord(x1_lower + 1, y),
                            dir: Coord(1, 0),
                            length: x1_upper - (x1_lower + 1),
                            room_left: room_index[[rx, ry]],
                            room_right: room_index[[rx + 1, ry - 1]],
                            next_matching: i,
                            door: false,
                        }
                    );
                }
            }

            adjacency_rows.push(adjacency_row);
        }

        {
            let mut adjacency_row: Vec<usize> = Vec::with_capacity(rooms_x);

            let ry = rooms_y;

            for rx in 0..rooms_x {
                let x0 = offset_x[[rx, ry-1]];
                let x1 = offset_x[[rx+1, ry-1]];
                let y = offset_y[[rx, ry]];

                let i = adjacencies.len();
                adjacency_row.push(i);

                adjacencies.push(
                    Adjacency {
                        origin: Coord(x0 + 1, y),
                        dir: Coord(1, 0),
                        length: x1 - (x0 + 1),
                        room_left: 0,
                        room_right: room_index[[rx, ry - 1]],
                        next_matching: i,
                        door: false,
                    }
                );
            }

            adjacency_rows.push(adjacency_row);
        }

        if mirror_x {
            for ry in 0..adjacency_rows.len() {
                let row = &adjacency_rows[ry];

                let mut i = 0;
                let mut j = row.len() - 1;
                while i < j {
                    let adj0 = row[i];
                    let adj1 = row[j];

                    adjacencies[adj0].next_matching = adj1;
                    adjacencies[adj1].next_matching = adj0;

                    // Flip edge a1 to point the opposite direction
                    {
                        let a1 = &mut adjacencies[adj1];
                        a1.origin += a1.dir * (a1.length - 1);
                        a1.dir = -a1.dir;
                        swap(&mut a1.room_left, &mut a1.room_right);
                    }

                    i += 1;
                    j -= 1;
                }
            }
        }

        if mirror_y {
            let mut ry0 = 0;
            let mut ry1 = adjacency_rows.len() - 1;
            while ry0 < ry1 {
                let row0 = &adjacency_rows[ry0];
                let row1 = &adjacency_rows[ry1];

                assert!(row0.len() == row1.len());

                for i in 0..row0.len() {
                    let adj0 = row0[i];
                    let adj1 = row1[i];
                    adjacencies[adj0].next_matching = adj1;
                    adjacencies[adj1].next_matching = adj0;
                }

                ry0 += 1;
                ry1 -= 1;
            }
        }
    }

    {
        let mut adjacency_rows: Vec<Vec<usize>> = Vec::with_capacity(rooms_x + 1);

        {
            let mut adjacency_row: Vec<usize> = Vec::with_capacity(rooms_y);

            let rx = 0;

            for ry in 0..rooms_y {
                let y0 = offset_y[[rx, ry]];
                let y1 = offset_y[[rx, ry+1]];
                let x = offset_x[[rx, ry]];

                let i = adjacencies.len();
                adjacency_row.push(i);

                adjacencies.push(
                    Adjacency {
                        origin: Coord(x, y0 + 1),
                        dir: Coord(0, 1),
                        length: y1 - (y0 + 1),
                        room_left: 0,
                        room_right: room_index[[rx, ry]],
                        next_matching: i,
                        door: false,
                    }
                );
            }

            adjacency_rows.push(adjacency_row);
        }

        for rx in 1..rooms_x {
            let mut adjacency_row: Vec<usize> = Vec::with_capacity(3 * rooms_y);

            for ry in 0..rooms_y {
                let y0_left  = offset_y[[rx-1, ry]];
                let y0_right = offset_y[[rx, ry]];
                let y1_left  = offset_y[[rx-1, ry+1]];
                let y1_right = offset_y[[rx, ry+1]];
                let y0 = max(y0_left, y0_right);
                let y1 = min(y1_left, y1_right);
                let x = offset_x[[rx, ry]];

                if ry > 0 && y0_left - y0_right > 1 {
                    let i = adjacencies.len();
                    adjacency_row.push(i);

                    adjacencies.push(
                        Adjacency {
                            origin: Coord(x, y0_right + 1),
                            dir: Coord(0, 1),
                            length: y0_left - (y0_right + 1),
                            room_left: room_index[[rx - 1, ry - 1]],
                            room_right: room_index[[rx, ry]],
                            next_matching: i,
                            door: false,
                        }
                    );
                }

                if y1 - y0 > 1 {
                    let i = adjacencies.len();
                    adjacency_row.push(i);

                    adjacencies.push(
                        Adjacency {
                            origin: Coord(x, y0 + 1),
                            dir: Coord(0, 1),
                            length: y1 - (y0 + 1),
                            room_left: room_index[[rx - 1, ry]],
                            room_right: room_index[[rx, ry]],
                            next_matching: i,
                            door: false,
                        }
                    );
                }

                if ry + 1 < rooms_y && y1_right - y1_left > 1 {
                    let i = adjacencies.len();
                    adjacency_row.push(i);

                    adjacencies.push(
                        Adjacency {
                            origin: Coord(x, y1_left + 1),
                            dir: Coord(0, 1),
                            length: y1_right - (y1_left + 1),
                            room_left: room_index[[rx - 1, ry + 1]],
                            room_right: room_index[[rx, ry]],
                            next_matching: i,
                            door: false,
                        }
                    );
                }
            }

            adjacency_rows.push(adjacency_row);
        }

        {
            let mut adjacency_row: Vec<usize> = Vec::with_capacity(rooms_y);

            let rx = rooms_x;

            for ry in 0..rooms_y {
                let y0 = offset_y[[rx-1, ry]];
                let y1 = offset_y[[rx-1, ry+1]];
                let x = offset_x[[rx, ry]];

                let i = adjacencies.len();
                adjacencies.push(
                    Adjacency {
                        origin: Coord(x, y0 + 1),
                        dir: Coord(0, 1),
                        length: y1 - (y0 + 1),
                        room_left: room_index[[rx - 1, ry]],
                        room_right: 0,
                        next_matching: i,
                        door: false,
                    }
                );
                adjacency_row.push(i);
            }

            adjacency_rows.push(adjacency_row);
        }

        if mirror_y {
            for ry in 0..adjacency_rows.len() {
                let row = &adjacency_rows[ry];
                let n = row.len() / 2;

                for i in 0..n {
                    let adj0 = row[i];
                    let adj1 = row[(row.len() - 1) - i];

                    adjacencies[adj0].next_matching = adj1;
                    adjacencies[adj1].next_matching = adj0;

                    {
                        // Flip edge a1 to point the opposite direction
                        let a1 = &mut adjacencies[adj1];
                        a1.origin += a1.dir * (a1.length - 1);
                        a1.dir = -a1.dir;
                        swap(&mut a1.room_left, &mut a1.room_right);
                    }
                }
            }
        }

        if mirror_x {
            let mut ry0 = 0;
            let mut ry1 = adjacency_rows.len() - 1;
            while ry0 < ry1 {
                let row0 = &adjacency_rows[ry0];
                let row1 = &adjacency_rows[ry1];

                assert!(row0.len() == row1.len());

                for i in 0..row0.len() {
                    let adj0 = row0[i];
                    let adj1 = row1[i];
                    adjacencies[adj0].next_matching = adj1;
                    adjacencies[adj1].next_matching = adj0;
                }

                ry0 += 1;
                ry1 -= 1;
            }
        }
    }

    adjacencies
}

fn store_adjacencies_in_rooms(adjacencies: &[Adjacency], rooms: &mut [Room]) {
    for (i, adj) in adjacencies.iter().enumerate() {
        let i0 = adj.room_left;
        let i1 = adj.room_right;
        rooms[i0].edges.push(i);
        rooms[i1].edges.push(i);
    }
}

fn get_edge_sets(random: &mut Random, adjacencies: &[Adjacency]) -> Vec<Vec<usize>> {
    let mut edge_sets = Vec::with_capacity(adjacencies.len());

    for (i, adj) in adjacencies.iter().enumerate() {
        let j = adj.next_matching;
        if j >= i {
            if j > i {
                edge_sets.push(vec![i, j]);
            } else {
                edge_sets.push(vec![i]);
            }
        }
    }

    edge_sets.shuffle(random);

    edge_sets
}

fn connect_rooms(random: &mut Random, rooms: &mut [Room], adjacencies: &mut [Adjacency]) -> Coord {

    // Collect sets of edges that are mirrors of each other

    let edge_sets = get_edge_sets(random, &adjacencies);

    // Connect all adjacent courtyard rooms together.

    for adj in adjacencies.iter_mut() {
        let i0 = adj.room_left;
        let i1 = adj.room_right;
        if rooms[i0].room_type != RoomType::PublicCourtyard || rooms[i1].room_type != RoomType::PublicCourtyard {
            continue;
        }

        adj.door = true;
        let group0 = rooms[i0].group;
        let group1 = rooms[i1].group;
        join_groups(rooms, group0, group1);
    }

    // Connect all the interior rooms with doors.

    for edge_set in &edge_sets {

        let mut added_door = false;

        {
            let adj = &mut adjacencies[edge_set[0]];

            let i0 = adj.room_left;
            let i1 = adj.room_right;

            if rooms[i0].room_type != RoomType::PublicRoom || rooms[i1].room_type != RoomType::PublicRoom {
                continue;
            }

            let group0 = rooms[i0].group;
            let group1 = rooms[i1].group;

            if group0 != group1 || random.gen_bool(0.4) {
                adj.door = true;
                added_door = true;
                join_groups(rooms, group0, group1);
            }
        }

        if added_door {
            for i in 1..edge_set.len() {
                let adj = &mut adjacencies[edge_set[i]];

                let i0 = adj.room_left;
                let i1 = adj.room_right;

                let group0 = rooms[i0].group;
                let group1 = rooms[i1].group;

                adj.door = true;
                join_groups(rooms, group0, group1);
            }
        }
    }

    // Create doors between the interiors and the courtyard areas.

    for edge_set in &edge_sets {

        let mut added_door = false;

        {
            let adj = &mut adjacencies[edge_set[0]];

            let i0 = adj.room_left;
            let i1 = adj.room_right;

            let room_type0 = rooms[i0].room_type;
            let room_type1 = rooms[i1].room_type;

            if room_type0 == room_type1 {
                continue;
            }

            if room_type0 == RoomType::Exterior || room_type1 == RoomType::Exterior {
                continue;
            }

            let group0 = rooms[i0].group;
            let group1 = rooms[i1].group;

            if group0 != group1 || random.gen_bool(0.4) {
                adj.door = true;
                added_door = true;
                join_groups(rooms, group0, group1);
            }
        }

        if added_door {
            for i in 1..edge_set.len() {
                let adj = &mut adjacencies[edge_set[i]];

                let i0 = adj.room_left;
                let i1 = adj.room_right;

                let group0 = rooms[i0].group;
                let group1 = rooms[i1].group;

                adj.door = true;
                join_groups(rooms, group0, group1);
            }
        }
    }

    // Create the door to the surrounding exterior. It must be on the south side.

    let mut pos_start = Coord(0, 0);

    {
        let i = front_door_adjacency_index(rooms, adjacencies, &edge_sets);

        // Set the player's start position based on where the door is.

        pos_start.0 = adjacencies[i].origin.0 + adjacencies[i].dir.0 * (adjacencies[i].length / 2);
        pos_start.1 = OUTER_BORDER - 1;

        adjacencies[i].door = true;

        // Break symmetry if the door is off center.

        let j = adjacencies[i].next_matching;
        if j != i {
            adjacencies[j].next_matching = j;
            adjacencies[i].next_matching = i;
        }
    }

    pos_start
}

fn front_door_adjacency_index(rooms: &[Room], adjacencies: &[Adjacency], edge_sets: &[Vec<usize>]) -> usize {
    for edge_set in edge_sets {
        for &i in edge_set {
            let adj = &adjacencies[i];

            if adj.dir.0 == 0 {
                continue;
            }

            if adj.next_matching > i {
                continue;
            }

            if adj.next_matching == i {
                if rooms[adj.room_right].room_type != RoomType::Exterior {
                    continue;
                }
            } else {
                if rooms[adj.room_left].room_type != RoomType::Exterior {
                    continue;
                }
            }

            return i;
        }
    }

    // Should always return above...

    0
}

fn join_groups(rooms: &mut [Room], group_from: usize, group_to: usize) {
    if group_from != group_to {
        for room in rooms.iter_mut() {
            if room.group == group_from {
                room.group = group_to;
            }
        }
    }
}

fn assign_room_types(room_index: &Array2D<usize>, adjacencies: &[Adjacency], rooms: &mut [Room]) {

    // Assign rooms depth based on distance from the bottom row of rooms.

    let unvisited = rooms.len();

    rooms[0].depth = 0;

    for i in 1..rooms.len() {
        rooms[i].depth = unvisited;
    }

    let mut rooms_to_visit: Vec<usize> = Vec::with_capacity(rooms.len());

    for x in 0..room_index.extents()[0] {
        let i_room = room_index[[x, 0]];
        rooms[i_room].depth = 1;
        rooms_to_visit.push(i_room);
    }

    // Visit rooms in breadth-first order, assigning them distances from the seed rooms.

    let mut ii_room = 0;
    while ii_room < rooms_to_visit.len() {
        let i_room = rooms_to_visit[ii_room];

        for i_adj in &rooms[i_room].edges.clone() {
            let adj: &Adjacency = &adjacencies[*i_adj];

            if !adj.door {
                continue;
            }

            let i_room_neighbor = if adj.room_left == i_room {adj.room_right} else {adj.room_left};

            if rooms[i_room_neighbor].depth == unvisited {
                rooms[i_room_neighbor].depth = rooms[i_room].depth + 1;
                rooms_to_visit.push(i_room_neighbor);
            }
        }

        ii_room += 1;
    }

    // Assign master-suite room type to the inner rooms.

    let mut max_depth = 0;
    for room in rooms.iter() {
        max_depth = max(max_depth, room.depth);
    }

    let target_num_master_rooms = (room_index.extents()[0] * room_index.extents()[1]) / 4;

    let mut num_master_rooms = 0;

    let mut depth = max_depth;
    while depth > 0 {
        for room in rooms.iter_mut() {
            if room.room_type != RoomType::PublicRoom && room.room_type != RoomType::PublicCourtyard {
                continue;
            }

            if room.depth != depth {
                continue;
            }

            room.room_type = if room.room_type == RoomType::PublicRoom {RoomType::PrivateRoom} else {RoomType::PrivateCourtyard};
            if room.room_type == RoomType::PrivateRoom {
                num_master_rooms += 1;
            }
        }

        if num_master_rooms >= target_num_master_rooms {
            break;
        }

        depth -= 1;
    }

    // Change any public courtyards that are adjacent to private courtyards into private courtyards

    loop {
        let mut changed = false;

        for i_room in 0..rooms.len() {
            if rooms[i_room].room_type != RoomType::PublicCourtyard {
                continue;
            }

            for i_adj in &rooms[i_room].edges {
                let adj = &adjacencies[*i_adj];

                let i_room_other = if adj.room_left != i_room {adj.room_left} else {adj.room_right};

                if rooms[i_room_other].room_type == RoomType::PrivateCourtyard {
                    rooms[i_room].room_type = RoomType::PrivateCourtyard;
                    changed = true;
                    break;
                }
            }
        }

        if !changed {
            break;
        }
    }
}

const ONE_WAY_WINDOW: [CellType; 5] = [
    CellType::OneWayWindowS,
    CellType::OneWayWindowE,
    CellType::OneWayWindowE, // not used
    CellType::OneWayWindowW,
    CellType::OneWayWindowN,
];

fn is_courtyard_room_type(room_type: RoomType) -> bool {
    match room_type {
        RoomType::Exterior => false,
        RoomType::PublicCourtyard => true,
        RoomType::PublicRoom => false,
        RoomType::PrivateCourtyard => true,
        RoomType::PrivateRoom => false,
    }
}

fn render_walls(random: &mut Random, rooms: &[Room], adjacencies: &[Adjacency], map: &mut Map) {

    // Render grass connecting courtyard rooms.

    for adj in adjacencies.iter() {
        let type0 = rooms[adj.room_left].room_type;
        let type1 = rooms[adj.room_right].room_type;

        if !is_courtyard_room_type(type0) || !is_courtyard_room_type(type1) {
            continue;
        }

        for j in 0..adj.length {
            let p: Coord = adj.origin + adj.dir * j;
            map.cells[[p.0 as usize, p.1 as usize]].cell_type = CellType::GroundGrass;
        }
    }

    // Render doors and windows for the rest of the walls.

    for i in 0..adjacencies.len() {
        let adj0 = &adjacencies[i];

        let type0 = rooms[adj0.room_left].room_type;
        let type1 = rooms[adj0.room_right].room_type;

        if is_courtyard_room_type(type0) && is_courtyard_room_type(type1) {
            continue;
        }

        let j = adj0.next_matching;

        if j < i {
            continue;
        }

        let offset =
            if j == i {
                adj0.length / 2
            } else if adj0.length > 2 {
                1 + random.gen_range(0..adj0.length - 2)
            } else {
                random.gen_range(0..adj0.length)
            };

        let mut walls: Vec<&Adjacency> = Vec::with_capacity(2);
        walls.push(adj0);

        if j != i {
            walls.push(&adjacencies[j]);
        }

        if !adj0.door && type0 != type1 {
            if type0 == RoomType::Exterior || type1 == RoomType::Exterior {
                if (adj0.length & 1) != 0 {
                    let k = adj0.length / 2;

                    for a in &walls {
                        let p = a.origin + a.dir * k;

                        let dir =
                            if rooms[a.room_right].room_type == RoomType::Exterior {
                                -a.dir
                            } else {
                                a.dir
                            };

                        map.cells[[p.0 as usize, p.1 as usize]].cell_type = ONE_WAY_WINDOW[(2 * dir.0 + dir.1 + 2) as usize];
                    }
                }
            } else if is_courtyard_room_type(type0) || is_courtyard_room_type(type1) {
                let mut k = random.gen_range(0..2);
                let k_end = (adj0.length + 1) / 2;

                while k < k_end {
                    for a in &walls {
                        let dir =
                            if is_courtyard_room_type(rooms[a.room_right].room_type) {
                                -a.dir
                            } else {
                                a.dir
                            };

                        let window_type = ONE_WAY_WINDOW[(2 * dir.0 + dir.1 + 2) as usize];

                        let p: Coord = a.origin + a.dir * k;
                        let q: Coord = a.origin + a.dir * (a.length - (k + 1));

                        map.cells[[p.0 as usize, p.1 as usize]].cell_type = window_type;
                        map.cells[[q.0 as usize, q.1 as usize]].cell_type = window_type;
                    }
                    k += 2;
                }
            }
        }

        let install_master_suite_door = random.gen_bool(0.3333);

        for a in &walls {
            if !a.door {
                continue;
            }

            let p = a.origin + a.dir * offset;

            let orient_ns = a.dir.0 == 0;

            map.cells[[p.0 as usize, p.1 as usize]].cell_type = if orient_ns {CellType::DoorNS} else {CellType::DoorEW};

            let room_type_left = rooms[a.room_left].room_type;
            let room_type_right = rooms[a.room_right].room_type;

            if room_type_left == RoomType::Exterior || room_type_right == RoomType::Exterior {
                map.cells[[p.0 as usize, p.1 as usize]].cell_type = if orient_ns {CellType::PortcullisNS} else {CellType::PortcullisEW};
                place_item(map, p.0, p.1, if orient_ns {ItemKind::PortcullisNS} else {ItemKind::PortcullisEW});
            } else if room_type_left != RoomType::PrivateRoom || room_type_right != RoomType::PrivateRoom || install_master_suite_door {
                map.cells[[p.0 as usize, p.1 as usize]].cell_type = if orient_ns {CellType::DoorNS} else {CellType::DoorEW};
                place_item(map, p.0, p.1, if orient_ns {ItemKind::DoorNS} else {ItemKind::DoorEW});
            }
        }
    }
}

fn render_rooms(_level: usize, rooms: &[Room], map: &mut Map, random: &mut Random) {
    for i_room in 1..rooms.len() {
        let room = &rooms[i_room];

        let cell_type = match room.room_type {
            RoomType::Exterior => CellType::GroundNormal,
            RoomType::PublicCourtyard => CellType::GroundGrass,
            RoomType::PublicRoom => CellType::GroundWood,
            RoomType::PrivateCourtyard => CellType::GroundGrass,
            RoomType::PrivateRoom => CellType::GroundMarble,
        };

        for x in room.pos_min.0..room.pos_max.0 {
            for y in room.pos_min.1..room.pos_max.1 {
                /*
                let t =
                    if cell_type == CellType::GroundWood && level > 3 && random.gen_bool(1.0 / 50.0) {
                        CellType::GroundWoodCreaky
                    } else {
                        cell_type
                    };
                */

                map.cells[[x as usize, y as usize]].cell_type = cell_type; // t;
            }
        }

        if room.room_type == RoomType::PrivateCourtyard || room.room_type == RoomType::PrivateRoom {
            for x in room.pos_min.0 - 1 .. room.pos_max.0 + 1 {
                for y in room.pos_min.1 - 1 .. room.pos_max.1 + 1 {
                    map.cells[[x as usize, y as usize]].inner = true;
                }
            }
        }

        let dx = room.pos_max.0 - room.pos_min.0;
        let dy = room.pos_max.1 - room.pos_min.1;

        if is_courtyard_room_type(room.room_type) {
            if dx >= 5 && dy >= 5 {
                for x in room.pos_min.0 + 1 .. room.pos_max.0 - 1 {
                    for y in room.pos_min.1 + 1 .. room.pos_max.1 - 1 {
                        map.cells[[x as usize, y as usize]].cell_type = CellType::GroundWater;
                    }
                }
            } else if dx >= 2 && dy >= 2 {
                try_place_bush(map, room.pos_min.0, room.pos_min.1);
                try_place_bush(map, room.pos_max.0 - 1, room.pos_min.1);
                try_place_bush(map, room.pos_min.0, room.pos_max.1 - 1);
                try_place_bush(map, room.pos_max.0 - 1, room.pos_max.1 - 1);
            }
        } else if room.room_type == RoomType::PublicRoom || room.room_type == RoomType::PrivateRoom {
            if dx >= 5 && dy >= 5 {
                if room.room_type == RoomType::PrivateRoom {
                    for x in 2..dx-2 {
                        for y in 2..dy-2 {
                            map.cells[[(room.pos_min.0 + x) as usize, (room.pos_min.1 + y) as usize]].cell_type = CellType::GroundWater;
                        }
                    }
                }

                map.cells[[(room.pos_min.0 + 1) as usize, (room.pos_min.1 + 1) as usize]].cell_type = CellType::Wall0000;
                map.cells[[(room.pos_max.0 - 2) as usize, (room.pos_min.1 + 1) as usize]].cell_type = CellType::Wall0000;
                map.cells[[(room.pos_min.0 + 1) as usize, (room.pos_max.1 - 2) as usize]].cell_type = CellType::Wall0000;
                map.cells[[(room.pos_max.0 - 2) as usize, (room.pos_max.1 - 2) as usize]].cell_type = CellType::Wall0000;
            } else if dx == 5 && dy >= 3 && (room.room_type == RoomType::PublicRoom || random.gen_bool(1.0 / 3.0)) {
                for y in 1..dy-1 {
                    place_item(map, room.pos_min.0 + 1, room.pos_min.1 + y, ItemKind::Chair);
                    place_item(map, room.pos_min.0 + 2, room.pos_min.1 + y, ItemKind::Table);
                    place_item(map, room.pos_min.0 + 3, room.pos_min.1 + y, ItemKind::Chair);
                }
            } else if dy == 5 && dx >= 3 && (room.room_type == RoomType::PublicRoom || random.gen_bool(1.0 / 3.0)) {
                for x in 1..dx-1 {
                    place_item(map, room.pos_min.0 + x, room.pos_min.1 + 1, ItemKind::Chair);
                    place_item(map, room.pos_min.0 + x, room.pos_min.1 + 2, ItemKind::Table);
                    place_item(map, room.pos_min.0 + x, room.pos_min.1 + 3, ItemKind::Chair);
                }
            } else if dx > dy && (dy & 1) == 1 && random.gen_bool(2.0 / 3.0) {
                let y = room.pos_min.1 + dy / 2;

                if room.room_type == RoomType::PublicRoom {
                    try_place_table(map, room.pos_min.0 + 1, y);
                    try_place_table(map, room.pos_max.0 - 2, y);
                } else {
                    try_place_chair(map, room.pos_min.0 + 1, y);
                    try_place_chair(map, room.pos_max.0 - 2, y);
                }
            } else if dy > dx && (dx & 1) == 1 && random.gen_bool(2.0 / 3.0) {
                let x = room.pos_min.0 + dx / 2;

                if room.room_type == RoomType::PublicRoom {
                    try_place_table(map, x, room.pos_min.1 + 1);
                    try_place_table(map, x, room.pos_max.1 - 2);
                } else {
                    try_place_chair(map, x, room.pos_min.1 + 1);
                    try_place_chair(map, x, room.pos_max.1 - 2);
                }
            } else if dx > 3 && dy > 3 {
                if room.room_type == RoomType::PublicRoom {
                    try_place_table(map, room.pos_min.0, room.pos_min.1);
                    try_place_table(map, room.pos_max.0 - 1, room.pos_min.1);
                    try_place_table(map, room.pos_min.0, room.pos_max.1 - 1);
                    try_place_table(map, room.pos_max.0 - 1, room.pos_max.1 - 1);
                } else {
                    try_place_chair(map, room.pos_min.0, room.pos_min.1);
                    try_place_chair(map, room.pos_max.0 - 1, room.pos_min.1);
                    try_place_chair(map, room.pos_min.0, room.pos_max.1 - 1);
                    try_place_chair(map, room.pos_max.0 - 1, room.pos_max.1 - 1);
                }
            }
        }
    }
}

fn door_adjacent(map: &CellGrid, x: i32, y: i32) -> bool {
    if map[[(x - 1) as usize, y as usize]].cell_type >= CellType::PortcullisNS {
        return true;
    }

    if map[[(x + 1) as usize, y as usize]].cell_type >= CellType::PortcullisNS {
        return true;
    }

    if map[[x as usize, (y - 1) as usize]].cell_type >= CellType::PortcullisNS {
        return true;
    }

    if map[[x as usize, (y + 1) as usize]].cell_type >= CellType::PortcullisNS {
        return true;
    }

    false
}

fn try_place_bush(map: &mut Map, x: i32, y: i32) {
    if map.cells[[x as usize, y as usize]].cell_type != CellType::GroundGrass {
        return;
    }

    if door_adjacent(&map.cells, x, y) {
        return;
    }

    place_item(map, x, y, ItemKind::Bush);
}

fn try_place_table(map: &mut Map, x: i32, y: i32) {
    if door_adjacent(&map.cells, x, y) {
        return;
    }

    place_item(map, x, y, ItemKind::Table);
}

fn try_place_chair(map: &mut Map, x: i32, y: i32) {
    if door_adjacent(&map.cells, x, y) {
        return;
    }

    place_item(map, x, y, ItemKind::Chair);
}

fn place_item(map: &mut Map, x: i32, y: i32, item_kind: ItemKind) {
    map.items.push(
        Item {
            pos: Coord(x, y),
            kind: item_kind,
        }
    );
}

fn place_outfits(random: &mut Random, rooms: &[Room], map: &mut Map) {

    let room_order = |room0: &&Room, room1: &&Room| {
        if room0.dead_end && !room1.dead_end {
            return Ordering::Less;
        } else if !room0.dead_end && room1.dead_end {
            return Ordering::Greater;
        }
        if room0.depth >= 2 && room1.depth < 2 {
            return Ordering::Less;
        } else if room0.depth < 2 && room1.depth >= 2 {
            return Ordering::Greater;
        }
        Ordering::Equal
    };

    let mut rooms_ordered: Vec<&Room> = rooms.iter().collect();
    rooms_ordered.retain(|room| room.room_type != RoomType::Exterior);
    rooms_ordered.shuffle(random);
    rooms_ordered.sort_by(room_order);

    let outfits = vec![guard::GuardKind::Outer, guard::GuardKind::Inner];
    let mut outfit_index = 0;

    let mut num_outfits: usize = max(1, min(outfits.len(), rooms.len() / 12));
    for room in rooms_ordered {
        if try_place_outfit(random, room.pos_min, room.pos_max, map, outfits[outfit_index]) {
            outfit_index += 1;
            num_outfits -= 1;
            if num_outfits == 0 {
                break;
            }
        }
    }
}

fn try_place_outfit(random: &mut Random, pos_min: Coord, pos_max: Coord, map: &mut Map, outfit_kind: guard::GuardKind) -> bool
{
    let dx = pos_max.0 - pos_min.0;
    let dy = pos_max.1 - pos_min.1;

    for _ in 0..1000 {
        let pos = Coord(pos_min.0 + random.gen_range(0..dx), pos_min.1 + random.gen_range(0..dy));

        let cell_type = map.cells[[pos.0 as usize, pos.1 as usize]].cell_type;

        if cell_type != CellType::GroundWood && cell_type != CellType::GroundMarble && cell_type != CellType::GroundGrass {
            continue;
        }

        if is_item_at_pos(&map, pos.0, pos.1) {
            continue;
        }

        if door_or_window_adjacent(&map.cells, pos) {
            continue;
        }
    
        place_item(map, pos.0, pos.1, ItemKind::Outfit(Some(outfit_kind)));
        return true;
    }

    false
}

fn door_or_window_adjacent(map: &CellGrid, pos: Coord) -> bool {
    for dir in &DIRS {
        let pos_adj = pos + *dir;
        if map[[pos_adj.0 as usize, pos_adj.1 as usize]].cell_type >= CellType::OneWayWindowE {
            return true;
        }
    }
    false
}

const DIRS: [Coord; 4] = [
    Coord(-1, 0),
    Coord(1, 0),
    Coord(0, -1),
    Coord(0, 1),
];

fn place_loot(random: &mut Random, rooms: &Vec<Room>, adjacencies: &[Adjacency], map: &mut Map) {

    // Count number of internal rooms.

    let mut num_rooms = 0;
    for room in rooms {
        if room.room_type == RoomType::PublicRoom || room.room_type == RoomType::PrivateRoom {
            num_rooms += 1;
        }
    }

    // Master-suite rooms get loot.

    for room in rooms  {
        if room.room_type != RoomType::PrivateRoom {
            continue;
        }

        if random.gen_bool(0.2) {
            continue;
        }

        try_place_loot(random, room.pos_min, room.pos_max, map);
    }

    // Dead-end rooms automatically get loot.

    for room in rooms.iter() {
        if room.room_type != RoomType::PublicRoom && room.room_type != RoomType::PrivateRoom {
            continue;
        }

        let mut num_exits = 0;
        for i_adj in room.edges.iter() {
            if adjacencies[*i_adj].door {
                num_exits += 1;
            }
        }

        if num_exits < 2 {
            try_place_loot(random, room.pos_min, room.pos_max, map);
        }
    }

    // Place a bit of extra loot.

    let pos_min = Coord(0, 0);
    let pos_max = Coord(map.cells.extents()[0] as i32, map.cells.extents()[1] as i32);
    for _ in 0..(num_rooms / 4 + random.gen_range(0..4)) {
        try_place_loot(random, pos_min, pos_max, map);
    }
}

fn is_item_at_pos(map: &Map, x: i32, y: i32) -> bool {
    for item in &map.items {
        if item.pos.0 == x && item.pos.1 == y {
            return true;
        }
    }
    for guard in &map.guards {
        if guard.pos.0 == x && guard.pos.1 == y {
            return true;
        }
    }
    return false;
}

fn try_place_loot(random: &mut Random, pos_min: Coord, pos_max: Coord, map: &mut Map)
{
    let dx = pos_max.0 - pos_min.0;
    let dy = pos_max.1 - pos_min.1;

    for _ in 0..1000 {
        let pos = Coord(pos_min.0 + random.gen_range(0..dx), pos_min.1 + random.gen_range(0..dy));

        let cell_type = map.cells[[pos.0 as usize, pos.1 as usize]].cell_type;

        if cell_type != CellType::GroundWood && cell_type != CellType::GroundMarble {
            continue;
        }

        if is_item_at_pos(&map, pos.0, pos.1) {
            continue;
        }

        place_item(map, pos.0, pos.1, ItemKind::Coin);
        break;
    }
}

fn place_exterior_bushes(random: &mut Random, map: &mut Map) {
    let sx = map.cells.extents()[0] as i32;
    let sy = map.cells.extents()[1] as i32;

    for x in 0..sx {
        for y in sy - OUTER_BORDER + 1 .. sy {
            if map.cells[[x as usize, y as usize]].cell_type != CellType::GroundNormal {
                continue;
            }

            let cell = &mut map.cells[[x as usize, y as usize]];
            cell.cell_type = CellType::GroundGrass;
            cell.seen = true;
        }

        if (x & 1) == 0 && random.gen_bool(0.8) {
            place_item(map, x, sy - 1, ItemKind::Bush);
        }
    }

    for y in OUTER_BORDER .. sy - OUTER_BORDER + 1 {
        for x in 0..OUTER_BORDER-1 {
            if map.cells[[x as usize, y as usize]].cell_type != CellType::GroundNormal {
                continue;
            }

            let cell = &mut map.cells[[x as usize, y as usize]];
            cell.cell_type = CellType::GroundGrass;
            cell.seen = true;
        }

        for x in (sx - OUTER_BORDER + 1) .. sx {
            if map.cells[[x as usize, y as usize]].cell_type != CellType::GroundNormal {
                continue;
            }

            let cell = &mut map.cells[[x as usize, y as usize]];
            cell.cell_type = CellType::GroundGrass;
            cell.seen = true;
        }

        if ((sy - y) & 1) != 0 {
            if random.gen_bool(0.8) {
                place_item(map, 0, y, ItemKind::Bush);
            }
            if random.gen_bool(0.8) {
                place_item(map, sx - 1, y, ItemKind::Bush);
            }
        }
    }
}

fn place_front_pillars(map: &mut Map) {
    let sx = (map.cells.extents()[0] as i32) - 1;
    let cx = (map.cells.extents()[0] as i32) / 2;

    let mut x = OUTER_BORDER;
    while x < cx {
        map.cells[[x as usize, 1]].cell_type = CellType::Wall0000;
        map.cells[[(sx - x) as usize, 1]].cell_type = CellType::Wall0000;
        x += 5;
    }
}

fn place_guards_by_type(random: &mut Random, level: usize, rooms: &[Room], map: &mut Map, guard_kind: guard::GuardKind) {

    let num_rooms = rooms.iter().filter(|room| room.patroller == Some(guard_kind)).count();

    // Generate guards

    let mut num_guards =
        if level == 1 && num_rooms > 0 {
            1
        } else {
            (num_rooms * min(level + 18, 40) + 99) / 100
        };

    while num_guards > 0 {
        if let Some(pos) = generate_initial_guard_pos(random, &map) {
            place_guard(random, map, pos, guard_kind);
            num_guards -= 1;
        }
    }
}

fn generate_initial_guard_pos(random: &mut Random, map: &Map) -> Option<Coord> {
    let size_x = map.cells.extents()[0] as i32;
    let size_y = map.cells.extents()[1] as i32;
    for _ in 0..1000 {
        let pos = Coord(random.gen_range(0..size_x), random.gen_range(0..size_y));

        let dpos = map.pos_start - pos;
        if dpos.length_squared() < 64 {
            continue;
        }

        let cell_type = map.cells[[pos.0 as usize, pos.1 as usize]].cell_type;

        if cell_type != CellType::GroundWood && cell_type != CellType::GroundMarble {
            continue;
        }

        if is_item_at_pos(&map, pos.0, pos.1) {
            continue;
        }

        return Some(pos);
    }

    return None;
}

fn place_guard(random: &mut Random, map: &mut Map, pos: Coord, kind: guard::GuardKind) {

    let mut guard = guard::Guard {
        pos: pos,
        dir: Coord(1, 0),
        kind: kind,
        mode: guard::GuardMode::Patrol,
        speaking: false,
        has_moved: false,
        heard_thief: false,
        hearing_guard: false,
        heard_guard: false,
        heard_guard_pos: pos,
        goal: pos,
        mode_timeout: 0,
        region_goal: INVALID_REGION,
        region_prev: INVALID_REGION,
    };

    guard.setup_goal_region(random, map);
    guard.dir = guard.initial_dir(map);

    map.guards.push(guard);
}

fn mark_exterior_as_seen(map: &mut Map) {
    let sx = map.cells.extents()[0];
    let sy = map.cells.extents()[1];

    for x in 0..sx {
        for y in 0..sy {
            if map.cells[[x, y]].cell_type == CellType::GroundNormal ||
                (x > 0 && map.cells[[x-1, y]].cell_type == CellType::GroundNormal) ||
                (x > 0 && y > 0 && map.cells[[x-1, y-1]].cell_type == CellType::GroundNormal) ||
                (x > 0 && y+1 < sy && map.cells[[x-1, y+1]].cell_type == CellType::GroundNormal) ||
                (y > 0 && map.cells[[x, y-1]].cell_type == CellType::GroundNormal) ||
                (y+1 < sy && map.cells[[x, y+1]].cell_type == CellType::GroundNormal) ||
                (x+1 < sx && map.cells[[x+1, y]].cell_type == CellType::GroundNormal) ||
                (x+1 < sx && y > 0 && map.cells[[x+1, y-1]].cell_type == CellType::GroundNormal) ||
                (x+1 < sx && y+1 < sy && map.cells[[x+1, y+1]].cell_type == CellType::GroundNormal) {
                map.cells[[x, y]].seen = true;
            }
        }
    }
}

fn cache_cell_info(map: &mut Map) {
    let sx = map.cells.extents()[0];
    let sy = map.cells.extents()[1];

    for x in 0..sx {
        for y in 0..sy {
            let cell = &mut map.cells[[x, y]];
            let cell_type = cell.cell_type;
            let tile = cell_grid::tile_def(cell_type);
            cell.move_cost = cell_grid::guard_move_cost_for_tile_type(cell_type);
            cell.blocks_player_sight = tile.blocks_player_sight;
            cell.blocks_sight = tile.blocks_sight;
            cell.blocks_sound = tile.blocks_sound;
            cell.hides_player = false;
        }
    }

    for item in &map.items {
        let cell = &mut map.cells[[item.pos.0 as usize, item.pos.1 as usize]];
        let kind = item.kind;
        cell.move_cost = max(cell.move_cost, cell_grid::guard_move_cost_for_item_kind(kind));
        if kind == ItemKind::DoorNS || kind == ItemKind::DoorEW {
            cell.blocks_player_sight = true;
        }
        if kind == ItemKind::DoorNS || kind == ItemKind::DoorEW || kind == ItemKind::PortcullisNS || kind == ItemKind::PortcullisEW || kind == ItemKind::Bush {
            cell.blocks_sight = true;
        }
        if kind == ItemKind::Table || kind == ItemKind::Bush {
            cell.hides_player = true;
        }
    }
}

fn non_dead_end_rooms<F>(rooms: &[Room], adjacencies: &[Adjacency], accept_room: F) -> Vec<bool> where F: Fn(&Room) -> bool {
    let mut include_room: Vec<bool> = rooms.iter().map(accept_room).collect();

    // Trim dead ends out repeatedly until no more can be trimmed.

    loop {
        let mut trimmed = false;

        for (i_room, room) in rooms.iter().enumerate() {
            if !include_room[i_room] {
                continue;
            }

            let mut num_exits = 0;
            for i_adj in &room.edges {
                let adj = &adjacencies[*i_adj];

                if !adj.door {
                    continue;
                }

                let i_room_other = if adj.room_left != i_room {adj.room_left} else {adj.room_right};

                if include_room[i_room_other] {
                    num_exits += 1;
                }
            }

            if num_exits < 2 {
                include_room[i_room] = false;
                trimmed = true;
            }
        }

        if !trimmed {
            break;
        }
    }

    include_room
}

fn generate_patrol_routes(map: &mut Map, rooms: &mut [Room], adjacencies: &[Adjacency]) {
    let general_non_dead_end_room = non_dead_end_rooms(rooms, adjacencies, |room| room.room_type != RoomType::Exterior);
    let outer_non_dead_end_room = non_dead_end_rooms(rooms, adjacencies, |room| room.room_type != RoomType::Exterior && room.room_type != RoomType::PrivateRoom && room.room_type != RoomType::PrivateCourtyard);

    // Generate patrol regions for included rooms.

    let mut room_patrol_region = vec![INVALID_REGION; rooms.len()];

    for i_room in 0..rooms.len() {
        rooms[i_room].dead_end = !general_non_dead_end_room[i_room];
        if general_non_dead_end_room[i_room] {
            let inner = !outer_non_dead_end_room[i_room];
            rooms[i_room].patroller = Some(if inner {guard::GuardKind::Inner} else {guard::GuardKind::Outer});
            room_patrol_region[i_room] = add_patrol_region(map, rooms[i_room].pos_min, rooms[i_room].pos_max, inner);
        }
    }

    // Add connections between included rooms.

    for adj in adjacencies {
        if !adj.door {
            continue;
        }

        let region0 = room_patrol_region[adj.room_left];
        let region1 = room_patrol_region[adj.room_right];

        if region0 == INVALID_REGION || region1 == INVALID_REGION {
            continue;
        }

        add_patrol_route(map, region0, region1);
    }
}

fn add_patrol_region(map: &mut Map, pos_min: Coord, pos_max: Coord, inner: bool) -> usize {
    let i_patrol_region = map.patrol_regions.len();

    map.patrol_regions.push(
        PatrolRegion {
            rect: Rect {
                pos_min,
                pos_max,
            },
            inner,
        }
    );

    // Plot the region into the map.

    for x in pos_min.0..pos_max.0 {
        for y in pos_min.1..pos_max.1 {
            map.cells[[x as usize, y as usize]].region = i_patrol_region;
        }
    }

    i_patrol_region
}

fn add_patrol_route(map: &mut Map, region0: usize, region1: usize) {
    assert!(region0 < map.patrol_regions.len());
    assert!(region1 < map.patrol_regions.len());
    map.patrol_routes.push((region0, region1));
}
