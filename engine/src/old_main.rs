use std::borrow::Cow;
use std::thread::sleep;
use std::time::Duration;
use wgpu::SurfaceError;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;

mod visual;
mod renderer;

struct Bullet {
    pub instance: visual::ExternalInstRef<visual::PipePosition>,
    pub ttl: i32,
}

fn main() -> Result<(), String> {


    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("Pipe Cleaner", 800, 600)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;

    let renderer = renderer::Renderer::new(window, 72f32);


    let mut event_pump = sdl_context.event_pump()?;
    let mut left = 0f32;
    let mut right = 0f32;
    let frame_duration = Duration::from_secs_f64(1f64/144f64);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Window {
                    window_id,
                    win_event: WindowEvent::Resized(width, height),
                    ..
                } if window_id == window.id() => {
                    surf_config.width = width as u32;
                    surf_config.height = height as u32;
                    surface.configure(&device, &surf_config);
                    let depth_extent = wgpu::Extent3d {
                        width: width as u32,
                        height: height as u32,
                        depth_or_array_layers: 1,
                    };
                    depth_tex_desc.size = depth_extent;
                    depth_tex_view = device.create_texture(&depth_tex_desc)
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    cam.w_h_ratio = width as f32 / height as f32;
                }
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                Event::KeyDown {
                    keycode: Some(k),
                    ..
                } => {
                    if k == Keycode::A {
                        left = 1f32;
                    } else if k == Keycode::D {
                        right = 1f32;
                    } else if k == Keycode::O {
                        tmp_cubes.push(mgr.create_instance(
                            cube_cls,
                            visual::PipePosition {
                                angle: tmp_angle,
                                depth: 2.333f32,
                            },
                            [0f32, 1f32, 1f32],
                        ));
                        tmp_angle+= 0.1f32;
                    } else if k == Keycode::P {
                        if !tmp_cubes.is_empty() {
                            tmp_cubes.pop();
                        }
                    } else if k == Keycode::Space {
                        tmp_bullets.push(
                            Bullet {
                                instance: mgr.create_instance(
                                    bullet_cls,
                                    visual::PipePosition {
                                        angle: cube1.borrow().position.angle,
                                        depth: cube1.borrow().position.depth,
                                    },
                                    [1f32, 0f32, 1f32],
                                ),
                                ttl: 1000i32,
                            }
                        );
                    }
                }
                Event::KeyUp {
                    keycode: Some(k),
                    ..
                } => {
                    if k == Keycode::A {
                        left = 0f32;
                    } else if k == Keycode::D {
                        right = 0f32;
                    }
                }
                _e => {
                    //dbg!(e);
                }
            }
        }

        let velocity = (right - left) * 0.01f32;

        cube1.borrow_mut().position.angle+= velocity;


        tmp_bullets = tmp_bullets.into_iter()
            .map(|mut bullet| {
                bullet.ttl-= 1i32;
                bullet.instance.borrow_mut().position.depth+= 0.05f32;
                bullet
            })
            .filter(|bullet| bullet.ttl > 0)
            .collect();

        sleep(frame_duration);
    }

    Ok(())
}
