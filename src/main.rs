use std::borrow::Cow;
use std::thread::sleep;
use std::time::Duration;
use wgpu::SurfaceError;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;

mod visual;

struct Camera {
    pos: [f32; 3],
    vfov: f32,
    w_h_ratio: f32,
    far_z: f32,
}

impl Camera {
    fn world_to_screen(&self) -> [f32; 12] {
        let [x, y, z] = self.pos;
        [
            1f32, 0f32, 0f32, -x,
            0f32, 1f32, 0f32, -y,
            0f32, 0f32, 1f32, -z,
        ]
    }

    fn scale(&self) -> [f32; 3] {
        let half_h = (self.vfov / 2f32).tan();
        let half_w = self.w_h_ratio * half_h;
        [1f32/half_w, 1f32/half_h, 1f32/self.far_z]
    }
}

fn circle_pts(vert_count: i32) -> Vec<[f32; 3]> {
    (0..vert_count).map(|i| { 
        let angle = (i as f32)/(vert_count as f32) * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        [cos, sin, 0f32]
    }).collect()
}

fn loop_indices(vert_count: u32) -> Vec<u32> {
    (0u32..vert_count).flat_map(|idx| {
        [idx, (idx+1) % vert_count]
    }).collect()
}

pub fn cube_pts() -> Vec<[f32; 3]> {
    vec![
        [-0.1f32, -0.1f32, -0.1f32], // 0
        [-0.1f32, -0.1f32,  0.1f32], // 1
        [-0.1f32,  0.1f32, -0.1f32], // 2
        [-0.1f32,  0.1f32,  0.1f32], // 3
        [ 0.1f32, -0.1f32, -0.1f32], // 4
        [ 0.1f32, -0.1f32,  0.1f32], // 5
        [ 0.1f32,  0.1f32, -0.1f32], // 6
        [ 0.1f32,  0.1f32,  0.1f32], // 7
    ]
}

pub fn cube_indices() -> Vec<u32> {
    vec![
        0, 1,
        0, 2,
        0, 4,
        1, 3,
        1, 5,
        2, 3,
        2, 6,
        3, 7,
        4, 5,
        4, 6,
        5, 7,
        6, 7,
    ]
}


fn main() -> Result<(), String> {
    let mut cam = Camera {
        vfov: 75f32.to_radians(),
        w_h_ratio: 1f32,
        far_z: 20f32,
        pos: [0f32, 0f32, 0f32],
    };

    let cube_indices = cube_indices();
    let cube_verts = cube_pts();

    let cube_mesh = visual::Mesh {
        indices: &cube_indices[..],
        vertices: &cube_verts[..],
    };

    let circle_verts = circle_pts(20);
    let circle_indices = loop_indices(circle_verts.len() as u32);

    let circle_mesh = visual::Mesh {
        indices: &circle_indices[..],
        vertices: &circle_verts[..],
    };

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("Raw Window Handle Example", 800, 600)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let (width, height) = window.size();

    let instance = wgpu::Instance::default();
    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter_opt = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }));
    let adapter = match adapter_opt {
        Some(a) => a,
        None => return Err(String::from("No adapter found")),
    };

    let (device, queue) = match pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            limits: wgpu::Limits::default(),
            label: Some("device"),
            features: wgpu::Features::empty(),
        },
        None,
    )) {
        Ok(a) => a,
        Err(e) => return Err(e.to_string()),
    };

    let circle_ct = 20u64;
    let mut mgr_builder = visual::ManagerBuilder::new();
    let cube_cls = mgr_builder.register_class(cube_mesh);
    let circle_cls = mgr_builder.register_class(circle_mesh);
    let mut mgr = mgr_builder.build(200u32, &device);

    let cube1 = mgr.create_instance(
        cube_cls,
        visual::PipePosition {
            angle: 90f32.to_radians(),
            depth: 3f32,
        },
        [1f32, 0f32, 0f32],
    );

    let _cube2 = mgr.create_instance(cube_cls, visual::PipePosition {
        angle: 180f32.to_radians(),
        depth: 5f32,
    }, [0f32, 1f32, 0f32]);

    let _cube3 = mgr.create_instance(cube_cls, visual::PipePosition {
        angle: 45f32.to_radians(),
        depth: 4f32,
    }, [0f32, 0f32, 1f32]);

    let _circles: Vec<visual::ExternalInstRef<visual::WorldPosition>> = (0u64..circle_ct).into_iter()
        .map(|circle_idx| {
            let z = circle_idx as f32;
            let pos = visual::WorldPosition([0f32, 0f32, z]);
            mgr.create_instance(circle_cls, pos, [1f32, 1f32, 1f32])
        }).collect();

    let mut tmp_cubes = Vec::<visual::ExternalInstRef<visual::PipePosition>>::new();
    let mut tmp_angle = 0f32;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    cam.w_h_ratio = width as f32 / height as f32;
    let cam_bytes: Vec<u8> = cam.world_to_screen().into_iter().flat_map(|f| {
        f.to_ne_bytes()
    }).chain(
        cam.scale().into_iter().flat_map(|f| {
            f.to_ne_bytes()
        })
    ).chain([0u8, 0u8, 0u8, 0u8])
    .collect();
    
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniform_buffer"),
        size: cam_bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: true,
    });

    uniform_buffer.slice(..)
        .get_mapped_range_mut()
        .copy_from_slice(&cam_bytes);
    uniform_buffer.unmap();

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None
            }
        ],
        label: Some("bind_group_layout"),
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0u64,
                    size: None,
                }),
            },
        ],
        label: Some("bind_group"),
    });

    let vert_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 3]>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    };

    let inst_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x3,
        ],
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
        label: None,
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            buffers: &[vert_layout, inst_layout],
            module: &shader,
            entry_point: "vs_main",
        },
        fragment: Some(wgpu::FragmentState {
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            module: &shader,
            entry_point: "fs_main",
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        label: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    let depth_extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut depth_tex_desc = wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: depth_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[wgpu::TextureFormat::Depth24Plus],
    };

    let mut depth_tex_view = device.create_texture(&depth_tex_desc)
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut surf_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width,
        height,
        present_mode: wgpu::PresentMode::Mailbox,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
        ].to_vec(),
    };
    surface.configure(&device, &surf_config);

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

        let velocity = (right - left) * 0.003f32;

        cube1.borrow_mut().position.angle+= velocity;

        let cam_bytes: Vec<u8> = cam.world_to_screen().into_iter()
            .flat_map(|f| {
                 f.to_ne_bytes()
            }).chain(
                cam.scale().into_iter().flat_map(|f| {
                    f.to_ne_bytes()
                })
            ).chain([0u8, 0u8, 0u8, 0u8])
            .collect();

        queue.write_buffer(&uniform_buffer, 0u64, &cam_bytes);
        mgr.update(&queue);

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                let reason = match err {
                    SurfaceError::Timeout => "Timeout",
                    SurfaceError::Outdated => "Outdated",
                    SurfaceError::Lost => "Lost",
                    SurfaceError::OutOfMemory => "OutOfMemory",
                };
                panic!("Failed to get current surface texture! Reason: {}", reason)
            }
        };

        let out_color = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });

        {

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_color,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_tex_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1f32),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
                label: None,
            });

            let indices = mgr.indices();
            let vertices = mgr.vertices();
            let instances = mgr.instances();

            rpass.set_pipeline(&render_pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.set_index_buffer(
                indices.slice(..),
                wgpu::IndexFormat::Uint32
            );
            rpass.set_vertex_buffer(0, vertices.slice(..));
            rpass.set_vertex_buffer(1, instances.slice(..));

            mgr.ranges().for_each(|(idx_range, vtx_range)| {
                rpass.draw_indexed(
                    idx_range,
                    0,
                    vtx_range,
                );
            });
        }
        queue.submit([encoder.finish()]);
        frame.present();
        sleep(frame_duration);
    }

    Ok(())
}
