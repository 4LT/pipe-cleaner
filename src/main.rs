mod entity;
mod visual;
mod world;

use entity::PipePosition;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use std::thread::sleep;
use std::time::Duration;
use visual::geo;
use world::World;

static FRAME_DURATION: f64 = 1f64 / 120f64;

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

    let player_pos = PipePosition {
        angle: 3.0 * std::f32::consts::TAU / 4.0,
        depth: 1.0,
    };

    let player = world.place_entity(player_pos);

    {
        let mut player = player.borrow_mut();
        player.color = [0f32, 1f32, 1f32];
        player.model = cube_model;
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
                    }
                }
                Event::KeyUp {
                    keycode: Some(k), ..
                } => {
                    if k == Keycode::A {
                        left = 0.0;
                    } else if k == Keycode::D {
                        right = 0.0;
                    }
                }
                _ => {}
            };
        }

        let player_velocity = (right - left) * 0.03;

        {
            let mut player = player.borrow_mut();
            player.pos.angle += player_velocity;
        }

        rend.render((w, h), world.geometry());
        sleep(frame_duration);
    }

    Ok(())
}
