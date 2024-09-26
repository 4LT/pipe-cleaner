mod entity;
mod visual;
mod world;

use entity::{PipePosition, EntRef};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use std::thread::sleep;
use std::time::Duration;
use std::rc::Rc;
use visual::geo;
use world::World;

pub const FRAME_DURATION: f64 = 1f64 / 120f64;
pub const FRAME_DURATION_F32: f32 = FRAME_DURATION as f32;

fn main() -> Result<(), String> {
    let cube_vertices = geo::cube_pts();
    let cube_indices = geo::cube_indices();

    let cube_mesh = visual::Mesh {
        vertices: cube_vertices,
        indices: cube_indices,
    };

    let mut vis_mgr_builder = visual::ManagerBuilder::new();
    let mut world = World::new(&mut vis_mgr_builder, 20);
    let cube_model = vis_mgr_builder.register_class(cube_mesh);

    let bullet_think = move |world: &mut World, bullet: EntRef| {
        let mut countdown = bullet.borrow().countdown;
        
        if countdown > 0.0 {
            bullet.borrow_mut().countdown-= FRAME_DURATION;
        } else {
            world.remove_entity(bullet);
        }
    };

    let player_think = move |world: &mut World, player: EntRef| {
        if player.borrow().countdown > 0.0 {
            player.borrow_mut().countdown-= FRAME_DURATION;
        } else if player.borrow().fire {
            let bullet = world.place_entity(player.borrow().position);
            let mut bullet = bullet.borrow_mut();
            bullet.color = [1.0, 1.0, 0.0];
            bullet.model = cube_model;
            bullet.countdown = 3.0;
            let speed = 20.0;
            bullet.max_speed = speed;
            bullet.velocity = [0.0, speed];
            bullet.think = Rc::new(bullet_think);
            player.borrow_mut().countdown = 0.02;
        }
    };

    let player_pos = PipePosition {
        angle: 3.0 * std::f32::consts::TAU / 4.0,
        depth: 0.67,
    };

    let player = world.place_entity(player_pos);

    {
        let mut player = player.borrow_mut();
        player.color = [0f32, 1f32, 1f32];
        player.model = cube_model;
        player.max_acceleration = 80.0;
        player.max_speed = 8.0;
        player.think = Rc::new(player_think);
    }

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Pipecleaner", 800, 600)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let main_window_id = window.id();

    let mut rend =
        visual::Renderer::new(&window, 90.0, vis_mgr_builder).map_err(|e| e)?;

    let mut event_pump = sdl_context.event_pump()?;
    let frame_duration = Duration::from_secs_f64(FRAME_DURATION);

    let mut w = 800u32;
    let mut h = 600u32;
    let mut left = 0f32;
    let mut right = 0f32;
    let mut fire = false;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                }
                Event::Window {
                    window_id,
                    win_event: WindowEvent::Resized(width, height),
                    ..
                } if window_id == main_window_id => {
                    w = width as u32;
                    h = height as u32;
                }
                Event::KeyDown {
                    keycode: Some(k), ..
                } => {
                    if k == Keycode::A {
                        left = 1.0;
                    } else if k == Keycode::D {
                        right = 1.0;
                    } else if k == Keycode::SPACE {
                        fire = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(k), ..
                } => {
                    if k == Keycode::A {
                        left = 0.0;
                    } else if k == Keycode::D {
                        right = 0.0;
                    } else if k == Keycode::SPACE {
                        fire = false;
                    }
                }
                _ => {}
            };
        }

        {
            let mut player = player.borrow_mut();
            player.target_velocity[0] = (right - left) * player.max_speed;
            player.fire = fire;
        }

        world.update_logic();
        world.update_physics();
        rend.render((w, h), world.geometry());
        sleep(frame_duration);
    }

    Ok(())
}
