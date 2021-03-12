use crate::cell_grid::{CellType, INFINITE_COST, INVALID_REGION, Map, Player, Random};
use crate::color_preset;
use crate::coord::Coord;
use crate::speech_bubbles::Popups;

use multiarray::Array2D;
use rand::Rng;
use std::cmp::{min, max};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GuardMode
{
    Patrol,
    Look,
    LookAtDisguised,
    Listen,
    ChaseVisibleTarget,
    MoveToLastSighting,
    MoveToLastSound,
    MoveToGuardShout,
}

pub struct Guard {
    pub pos: Coord,
    pub dir: Coord,
    pub mode: GuardMode,
    pub speaking: bool,
    pub has_moved: bool,
    pub heard_thief: bool,
    pub hearing_guard: bool,
    pub heard_guard: bool,
    pub heard_guard_pos: Coord,

    // Chase
    pub goal: Coord,
    pub mode_timeout: usize,

    // Patrol
    pub region_goal: usize,
    pub region_prev: usize,
}

struct Shout {
    pos_shouter: Coord, // where is the person shouting?
    pos_target: Coord, // where are they reporting the player is?
}

pub fn guard_act_all(random: &mut Random, see_all: bool, popups: &mut Popups, lines: &mut Lines, map: &mut Map, player: &mut Player) {

    // Mark if we heard a guard last turn, and clear the speaking flag.

    for guard in map.guards.iter_mut() {
        guard.pre_turn();
    }

    // Update each guard for this turn.

    let mut guards = map.guards.split_off(0);
    let mut shouts: Vec<Shout> = Vec::new();

    for mut guard in guards.drain(..) {
        guard.act(random, see_all, popups, lines, player, map, &mut shouts);
        map.guards.push(guard);
    }

    // Process shouts

    for shout in &shouts {
        alert_nearby_guards(map, &shout);
    }
}

fn alert_nearby_guards(map: &mut Map, shout: &Shout) {
    for guard in map.guards_in_earshot(shout.pos_shouter, 150) {
        if guard.pos != shout.pos_shouter {
            guard.hear_guard(shout.pos_target);
        }
    }
}

fn pos_next_best(map: &Map, distance_field: &Array2D<usize>, pos_from: Coord) -> Coord {
    let mut cost_best = INFINITE_COST;
    let mut pos_best = pos_from;

    let pos_min = Coord(max(0, pos_from.0 - 1), max(0, pos_from.1 - 1));
    let pos_max = Coord(min(map.cells.extents()[0] as i32, pos_from.0 + 2), min(map.cells.extents()[1] as i32, pos_from.1 + 2));

    for x in pos_min.0 .. pos_max.0 {
        for y in pos_min.1 .. pos_max.1 {
            let cost = distance_field[[x as usize, y as usize]];
            if cost == INFINITE_COST {
                continue;
            }

            let pos = Coord(x, y);
            if map.guard_move_cost(pos_from, pos) == INFINITE_COST {
                continue;
            }

            if map.cells[[pos.0 as usize, pos.1 as usize]].cell_type == CellType::GroundWater {
                continue;
            }

            if map.is_guard_at(pos) {
                continue;
            }

            if cost < cost_best {
                cost_best = cost;
                pos_best = pos;
            }
        }
    }

    pos_best
}

struct LineIter {
    lines: &'static [&'static str],
    line_index: usize,
}

impl LineIter {
    fn new(lines: &'static [&'static str]) -> LineIter {
        LineIter { lines, line_index: 0 }
    }

    fn next(&mut self) -> &'static str {
        let s = self.lines[self.line_index];
        self.line_index = (self.line_index + 1) % self.lines.len();
        s
    }
}

pub struct Lines {
    see: LineIter,
    see_disguised: LineIter,
    hear: LineIter,
    hear_guard: LineIter,
    chase: LineIter,
    investigate: LineIter,
    end_chase: LineIter,
    end_investigate: LineIter,
    done_looking: LineIter,
    done_seeing_disguised: LineIter,
    done_listening: LineIter,
    damage: LineIter,
}

pub fn new_lines() -> Lines {
    Lines {
        see: LineIter::new(SEE_LINES),
        see_disguised: LineIter::new(SEE_DISGUISED_LINES),
        hear: LineIter::new(HEAR_LINES),
        hear_guard: LineIter::new(HEAR_GUARD_LINES),
        chase: LineIter::new(CHASE_LINES),
        investigate: LineIter::new(INVESTIGATE_LINES),
        end_chase: LineIter::new(END_CHASE_LINES),
        end_investigate: LineIter::new(END_INVESTIGATION_LINES),
        done_looking: LineIter::new(DONE_LOOKING_LINES),
        done_seeing_disguised: LineIter::new(DONE_SEEING_DISGUISED_LINES),
        done_listening: LineIter::new(DONE_LISTENING_LINES),
        damage: LineIter::new(DAMAGE_LINES),
    }
}

fn lines_for_state_change(lines: &mut Lines, mode_prev: GuardMode, mode_next: GuardMode) -> Option<&mut LineIter> {
    if mode_next == mode_prev {
        None
    } else {
        match mode_next {
            GuardMode::Patrol => {
                match mode_prev {
                    GuardMode::Look => Some(&mut lines.done_looking),
                    GuardMode::LookAtDisguised => Some(&mut lines.done_seeing_disguised),
                    GuardMode::Listen => Some(&mut lines.done_listening),
                    GuardMode::MoveToLastSound |
                    GuardMode::MoveToGuardShout => Some(&mut lines.end_investigate),
                    GuardMode::MoveToLastSighting => Some(&mut lines.end_chase),
                    _ => None
                }
            },
            GuardMode::Look => Some(&mut lines.see),
            GuardMode::LookAtDisguised => Some(&mut lines.see_disguised),
            GuardMode::Listen => Some(&mut lines.hear),
            GuardMode::ChaseVisibleTarget =>
                if mode_prev != GuardMode::MoveToLastSighting {Some(&mut lines.chase)} else {None},
            GuardMode::MoveToLastSighting => None,
            GuardMode::MoveToLastSound => Some(&mut lines.investigate),
            GuardMode::MoveToGuardShout => Some(&mut lines.hear_guard),
        }
    }
}

impl Guard {

fn pre_turn(&mut self) {
    self.heard_guard = self.hearing_guard;
    self.hearing_guard = false;
    self.speaking = false;
    self.has_moved = false;
}

pub fn hear_thief(&mut self) {
    self.heard_thief = true;
}

fn hear_guard(&mut self, pos_target: Coord) {
    self.hearing_guard = true;
    self.heard_guard_pos = pos_target;
}

fn act(&mut self, random: &mut Random, see_all: bool, popups: &mut Popups, lines: &mut Lines, player: &mut Player, map: &Map, shouts: &mut Vec<Shout>) {

    let mode_prev = self.mode;
    let pos_prev = self.pos;

    // See if senses will kick us into a new mode

    if self.sees_thief(map, player) {
        self.goal = player.pos;

        if self.mode == GuardMode::Patrol && (player.disguised || !self.adjacent_to(player.pos)) {
            self.mode = if player.disguised {GuardMode::LookAtDisguised} else {GuardMode::Look};
            self.mode_timeout = random.gen_range(3..6);
            self.dir = update_dir(self.dir, player.pos - self.pos);
        } else {
            self.mode = GuardMode::ChaseVisibleTarget;
        }
    } else if self.mode == GuardMode::ChaseVisibleTarget {
        self.mode = GuardMode::MoveToLastSighting;
        self.mode_timeout = 3;
        self.goal = player.pos;
    }

    if self.mode != GuardMode::ChaseVisibleTarget {
        if self.heard_guard {
            self.mode = GuardMode::MoveToGuardShout;
            self.mode_timeout = random.gen_range(2..6);
            self.goal = self.heard_guard_pos;
        }

        if self.heard_thief {
            if self.adjacent_to(player.pos) {
                self.mode = GuardMode::ChaseVisibleTarget;
                self.goal = player.pos;
            } else if self.mode == GuardMode::Patrol {
                self.mode = GuardMode::Listen;
                self.mode_timeout = random.gen_range(3..6);
                self.dir = update_dir(self.dir, player.pos - self.pos);
            } else {
                self.mode = GuardMode::MoveToLastSound;
                self.mode_timeout = random.gen_range(3..6);
                self.goal = player.pos;
            }
        }
    }

    // Pass time in the current mode

    match self.mode {
        GuardMode::Patrol => {
            self.patrol_step(map, player, random);
        },
        GuardMode::Look |
        GuardMode::LookAtDisguised |
        GuardMode::Listen => {
            self.mode_timeout -= 1;
            if self.mode_timeout == 0 {
                self.mode = GuardMode::Patrol;
            }
        },
        GuardMode::ChaseVisibleTarget => {
            if self.adjacent_to(player.pos) {
                self.dir = update_dir(self.dir, self.goal - self.pos);
                if mode_prev == GuardMode::ChaseVisibleTarget {
                    if !player.damaged_last_turn {
                        popups.damage(self.pos, lines.damage.next());
                    }
                    player.apply_damage(1);
                }
            } else {
                self.move_toward_goal(map, player);
            }
        },
        GuardMode::MoveToLastSighting |
        GuardMode::MoveToLastSound |
        GuardMode::MoveToGuardShout => {
            if !self.move_toward_goal(map, player) {
                self.mode_timeout -= 1;
            }

            if self.mode_timeout == 0 {
                self.mode = GuardMode::Patrol;
                self.setup_goal_region(random, map);
            }
        },
    }

    // If we moved, update state based on target visibility from new position

    if self.pos != pos_prev {
        if self.sees_thief(map, player) {
            self.goal = player.pos;

            if self.mode == GuardMode::Patrol && (player.disguised || !self.adjacent_to(player.pos)) {
                self.mode = if player.disguised {GuardMode::LookAtDisguised} else {GuardMode::Look};
                self.mode_timeout = random.gen_range(3..6);
            } else {
                self.mode = GuardMode::ChaseVisibleTarget;
            }

            self.dir = update_dir(self.dir, player.pos - self.pos);
        } else if self.mode == GuardMode::ChaseVisibleTarget {
            self.mode = GuardMode::MoveToLastSighting;
            self.mode_timeout = 3;
            self.goal = player.pos;
        }
    }

    // Clear heard-thief flag

    self.heard_thief = false;

    // Say something to indicate state changes

    if let Some(line_iter) = lines_for_state_change(lines, mode_prev, self.mode) {
        self.say(popups, player, see_all, line_iter.next());
    }

    if self.mode == GuardMode::ChaseVisibleTarget && mode_prev != GuardMode::ChaseVisibleTarget {
        shouts.push(Shout{pos_shouter: self.pos, pos_target: player.pos});
    }
}

pub fn overhead_icon_and_color(&self, map: &Map, player: &Player, see_all: bool) -> Option<(u32, u32)> {
    let cell = &map.cells[[self.pos.0 as usize, self.pos.1 as usize]];
    let visible = see_all || cell.seen || self.speaking;
    if !visible && (player.pos - self.pos).length_squared() > 25 {
        return None;
    }

    if self.mode == GuardMode::ChaseVisibleTarget {
        return Some((217, color_preset::LIGHT_YELLOW));
    }

    if self.mode != GuardMode::Patrol {
        return Some((216, color_preset::LIGHT_YELLOW));
    }

    None
}

fn say(&mut self, popups: &mut Popups, player: &Player, see_all: bool, msg: &'static str) {
    let d = self.pos - player.pos;
    let dist_squared = d.length_squared();

    if dist_squared < 200 || see_all {
        popups.guard_speech(self.pos, msg);
    }

    self.speaking = true;
}

fn adjacent_to(&self, pos: Coord) -> bool {
    let d = pos - self.pos;
    d.0.abs() < 2 && d.1.abs() < 2
}

fn sees_thief(&self, map: &Map, player: &Player) -> bool {
    let d = player.pos - self.pos;
    if self.dir.dot(d) < 0 {
        return false;
    }

    let thief_disguised = player.disguised && self.mode != GuardMode::ChaseVisibleTarget;

    let player_is_lit = !thief_disguised && map.cells[[player.pos.0 as usize, player.pos.1 as usize]].lit;

    let d2 = d.length_squared();
    if d2 >= self.sight_cutoff(player_is_lit) {
        return false;
    }

    if !player.hidden(map) && line_of_sight(map, self.pos, player.pos) {
        return true;
    }

    if self.mode != GuardMode::Patrol && d.0.abs() < 2 && d.1.abs() < 2 {
        return true;
    }

    return false;
}

fn cutoff_lit(&self) -> i32 {
    if self.mode == GuardMode::Patrol || self.mode == GuardMode::LookAtDisguised {40} else {75}
}

fn cutoff_unlit(&self) -> i32 {
    if self.mode == GuardMode::Patrol || self.mode == GuardMode::LookAtDisguised {3} else {33}
}

fn sight_cutoff(&self, lit_target: bool) -> i32 {
    if lit_target {self.cutoff_lit()} else {self.cutoff_unlit()}
}

fn patrol_step(&mut self, map: &Map, player: &mut Player, random: &mut Random) {
    let bumped_thief = self.move_toward_region(map, player);

    if map.cells[[self.pos.0 as usize, self.pos.1 as usize]].region == self.region_goal {
        let region_prev = self.region_prev;
        self.region_prev = self.region_goal;
        self.region_goal = map.random_neighbor_region(random, self.region_goal, region_prev);
    }

    if bumped_thief && !player.disguised {
        self.mode = GuardMode::ChaseVisibleTarget;
        self.goal = player.pos;
        self.dir = update_dir(self.dir, self.goal - self.pos);
    }
}

pub fn initial_dir(&self, map: &Map) -> Coord
{
    if self.region_goal == INVALID_REGION {
        return self.dir;
    }

    let distance_field = map.compute_distances_to_region(self.region_goal);

    let pos_next = pos_next_best(map, &distance_field, self.pos);

    update_dir(self.dir, pos_next - self.pos)
}

fn move_toward_region(&mut self, map: &Map, player: &Player) -> bool {
    if self.region_goal == INVALID_REGION {
        return false;
    }

    let distance_field = map.compute_distances_to_region(self.region_goal);

    let pos_next = pos_next_best(map, &distance_field, self.pos);

    if player.pos == pos_next {
        return true;
    }

    self.dir = update_dir(self.dir, pos_next - self.pos);
    self.pos = pos_next;

    false
}

fn move_toward_goal(&mut self, map: &Map, player: &Player) -> bool {
    let dist_field = map.compute_distances_to_position(self.goal);

    let pos_next = pos_next_best(map, &dist_field, self.pos);
    if pos_next == self.pos {
        return false;
    }

    self.dir = update_dir(self.dir, pos_next - self.pos);

    if player.pos == pos_next {
        return false;
    }

    self.pos = pos_next;
    true
}

pub fn setup_goal_region(&mut self, random: &mut Random, map: &Map) {
    let region_cur = map.cells[[self.pos.0 as usize, self.pos.1 as usize]].region;

    if self.region_goal != INVALID_REGION && region_cur == self.region_prev {
        return;
    }

    if region_cur == INVALID_REGION {
        self.region_goal = map.closest_region(self.pos);
    } else {
        self.region_goal = map.random_neighbor_region(random, region_cur, self.region_prev);
        self.region_prev = region_cur;
    }
}

}

pub fn update_dir(dir_forward: Coord, dir_aim: Coord) -> Coord {
    let dir_left = Coord(-dir_forward.1, dir_forward.0);

    let dot_forward = dir_forward.dot(dir_aim);
    let dot_left = dir_left.dot(dir_aim);

    if dot_forward.abs() >= dot_left.abs() {
        if dot_forward >= 0 {dir_forward} else {-dir_forward}
    } else if dot_left.abs() > dot_forward.abs() {
        if dot_left >= 0 {dir_left} else {-dir_left}
    } else if dot_forward > 0 {
        dir_forward
    } else {
        if dot_left >= 0 {dir_left} else {-dir_left}
    }
}

fn line_of_sight(map: &Map, from: Coord, to: Coord) -> bool {
    let mut x = from.0;
    let mut y = from.1;

    let dx = to.0 - x;
    let dy = to.1 - y;

    let mut ax = dx.abs();
    let mut ay = dy.abs();

    let x_inc = if dx > 0 {1} else {-1};
    let y_inc = if dy > 0 {1} else {-1};

    let mut error = ay - ax;

    let mut n = ax + ay - 1;

    ax *= 2;
    ay *= 2;

    while n > 0 {
        if error > 0 {
            y += y_inc;
            error -= ax;
        } else {
            x += x_inc;
            error += ay;
        }

        if map.blocks_sight(x, y) {
            return false;
        }

        n -= 1;
    }

    true
}

static SEE_LINES: &[&str] = &[
    "Who goes there?",
    "Huh?",
    "What?",
    "Wait...",
    "Who's that?",
    "Hey...",
    "Hmm...",
    "What moved?",
    "Did that shadow move?",
    "I see something...",
    "Hello?",
];

static SEE_DISGUISED_LINES: &[&str] = &[
    "Who are you?",
    "You don't look familiar!",
    "Do I know you?",
    "Wait...",
    "Hey...",
    "Let me see your face...",
    "Do you belong here?",
    "You are...?",
    "Are you new here?",
];

static HEAR_LINES: &[&str] = &[
    "Huh?",
    "What?",
    "Hark!",
    "A noise...",
    "I heard something.",
    "Hmm...",
    "Who goes there?",
    "What's that noise?",
    "I hear something...",
    "Hello?",
];

static HEAR_GUARD_LINES: &[&str] = &[
    "Where?",
    "I'm coming!",
    "Here I come!",
    "To arms!",
    "Where is he?",
];

static CHASE_LINES: &[&str] = &[
    "Halt!",
    "Hey!",
    "Aha!",
    "I see you!",
    "I'm coming!",
    "I'll get you!",
    "Just you wait...",
    "You won't get away!",
    "Oh no you don't...",
    "Get him!",
    "After him!",
    "Thief!",
];

static INVESTIGATE_LINES: &[&str] = &[
    "That noise again...",
    "I heard it again!",
    "Someone's there!",
    "Who could that be?",
    "There it is again!",
    "What was that?",
    "Better check it out...",
    "What keeps making those noises?",
    "That better be rats!",
    "Again?",
];

static END_CHASE_LINES: &[&str] = &[
    "(huff, huff)",
    "Where did he go?",
    "Lost him!",
    "Gone!",
    "Come back!",
    "Argh!",
    "He's not coming back.",
    "Blast!",
    "Next time!",
];

static END_INVESTIGATION_LINES: &[&str] = &[
    "Guess it was nothing.",
    "Wonder what it was?",
    "Better get back.",
    "It's quiet now.",
    "This is where I heard it...",
    "Nothing, now.",
];

static DONE_LOOKING_LINES: &[&str] = &[
    "Must have been rats.",
    "Too much coffee!",
    "I've got the jitters.",
    "Probably nothing.",
    "I thought I saw something.",
    "Oh well.",
    "Nothing.",
    "Can't see it now.",
    "I've been up too long.",
    "Seeing things, I guess.",
    "Hope it wasn't anything.",
    "Did I imagine that?",
];

static DONE_SEEING_DISGUISED_LINES: &[&str] = &[
    "Who was that?",
    "Huh...",
    "I wonder who that was?",
    "Oh well.",
    "I'm seeing things.",
    "I've been up too long.",
    "Seeing things, I guess.",
    "Probably new here.",
    "Better get back to it.",
    "Did I imagine that?",
    "Should I tell the boss?",
];

static DONE_LISTENING_LINES: &[&str] = &[
    "Must have been rats.",
    "Too much coffee!",
    "I've got the jitters.",
    "Probably nothing.",
    "I thought I heard something.",
    "Oh well.",
    "Nothing.",
    "Can't hear it now.",
    "I've been up too long.",
    "Hearing things, I guess.",
    "Hope it wasn't anything.",
    "Did I imagine that?",
];

static DAMAGE_LINES: &[&str] = &[
    "Oof!",
    "Krak!",
    "Pow!",
    "Urk!",
    "Smack!",
    "Bif!",
];
