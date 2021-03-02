//this represents our external js function
extern {
    fn put_tile(i: i32, x: i32, y: i32, r: f32, g: f32, b: f32, a: f32);
}

const KEY_LEFT: i32 = 37;
const KEY_UP: i32 = 38;
const KEY_RIGHT: i32 = 39;
const KEY_DOWN: i32 = 40;

struct World {
    view_width: i32,
    view_height: i32,
    player_position: (i32, i32)
}

static mut WORLD_GLOBAL: World = World {
	view_width: 0,
	view_height: 0,
	player_position: (1, 1),
};

//a simple safe wrapper around calling the JS function
fn draw_tile(i: i32, x: i32, y: i32, r: f32, g: f32, b: f32, a: f32) -> () {
    unsafe {
        put_tile(i, x, y, r, g, b, a);
    }
}

/*
fn make_rgb(r: u8, g: u8, b: u8) -> i32 {
	((r as i32) << 16) + ((g as i32) << 8) + (b as i32)
}
*/

fn draw_world(world: &World) {
    //draw grass
    for y in 0..world.view_height {
        for x in 0..world.view_width {
          draw_tile(132, x, y, 0.0, 0.68, 0.0, 1.0);
        }
    }

    //draw player
    draw_tile(208, world.player_position.0, world.player_position.1, 0.66, 0.66, 0.66, 1.0);
}

fn get_world() -> &'static mut World {
	unsafe {
		&mut WORLD_GLOBAL
	}
}

#[no_mangle]
pub fn start(width: i32, height: i32) -> () {
	let world = get_world();
	world.view_width = width;
	world.view_height = height;
	draw_world(world);
}

#[no_mangle]
pub fn key_down(c: i32) -> () {
	let modifier = match c {
		KEY_LEFT => (-1,0),
		KEY_RIGHT => (1,0),
		KEY_DOWN => (0,-1),
		KEY_UP => (0,1),
		_ => (0,0)
	};
	let world = get_world();
	world.player_position = (world.player_position.0+modifier.0,world.player_position.1+modifier.1);
	draw_world(world);
}
