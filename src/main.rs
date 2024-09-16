mod entity;
mod visual;
mod world;

use entity::PipePosition;
use sdl2::event::{Event, WindowEvent};
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
    let cube_model = vis_mgr_builder.register_class(cube_mesh);
    let mut ent_mgr = entity::Manager::default();
    let player = ent_mgr.create();

    {
        let mut player = player.borrow_mut();
        player.pos = PipePosition {
            angle: 0f32,
            depth: 1f32,
        };
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

    let mut rend = visual::Renderer::new(&window, 90f32, vis_mgr_builder)
        .map_err(|e| e)?;

    let mut event_pump = sdl_context.event_pump()?;
    let frame_duration = Duration::from_secs_f64(FRAME_DURATION);

    let mut w = 800u32;
    let mut h = 600u32;

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
                _ => {}
            };
        }

        rend.render((w, h), ent_mgr.iter());
        sleep(frame_duration);
    }

    Ok(())
}
