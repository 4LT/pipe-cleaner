use crate::visual;
use std::borrow::Cow;

pub struct Camera {
    pos: [f32; 3],
    vfov: f32,
    w_h_ratio: f32,
    far_z: f32,
}

impl Camera {
    fn world_to_screen(&self) -> [f32; 12] {
        let [x, y, z] = self.pos;
        [
            1f32, 0f32, 0f32, -x, 0f32, 1f32, 0f32, -y, 0f32, 0f32, 1f32, -z,
        ]
    }

    fn scale(&self) -> [f32; 3] {
        let half_h = (self.vfov / 2f32).tan();
        let half_w = self.w_h_ratio * half_h;
        [1f32 / half_w, 1f32 / half_h, 1f32 / self.far_z]
    }

    fn into_bytes(&self) -> Vec<u8> {
        self.world_to_screen()
            .into_iter()
            .flat_map(|f| f.to_ne_bytes())
            .chain(self.scale().into_iter().flat_map(|f| f.to_ne_bytes()))
            .chain([0u8, 0u8, 0u8, 0u8])
            .collect()
    }
}

pub struct Renderer<'a> {
    cam: Camera,
    res_mgr: visual::Manager,
    queue: wgpu::Queue,
    device: wgpu::Device,
    surface_config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface<'a>,
    depth_texture: wgpu::Texture,
    uniforms: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    window_dimensions: (u32, u32),
}

impl<'a> Renderer<'a> {
    pub fn new(
        window: &sdl3::video::Window,
        vfov: f32,
        mgr_builder: visual::ManagerBuilder,
    ) -> Result<Renderer<'a>, String> {
        let (width, height) = window.size();

        let backends =
            wgpu::Backends::from_env().unwrap_or_else(wgpu::Backends::all);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            flags: wgpu::InstanceFlags::empty(),
            backend_options: wgpu::BackendOptions {
                dx12: wgpu::Dx12BackendOptions {
                    shader_compiler: wgpu::Dx12Compiler::Fxc,
                },
                gl: wgpu::GlBackendOptions {
                    gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
                    fence_behavior: Default::default(),
                },
                noop: Default::default(),
            },
        });

        let surface = unsafe {
            let surf_targ =
                wgpu::SurfaceTargetUnsafe::from_window(window).unwrap();
            instance.create_surface_unsafe(surf_targ).unwrap()
        };

        let adapter_opt = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ));
        let adapter = match adapter_opt {
            Ok(a) => a,
            Err(_) => return Err(String::from("No adapter found")),
        };

        let (device, queue) = match pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::default(),
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                memory_hints: Default::default(),
                trace: Default::default(),
            },
        )) {
            Ok(a) => a,
            Err(e) => return Err(e.to_string()),
        };

        let shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "shader.wgsl"
                ))),
            });

        let cam = Camera {
            vfov: vfov.to_radians(),
            w_h_ratio: width as f32 / height as f32,
            far_z: 20f32,
            pos: [0f32, 0f32, 0f32],
        };

        let cam_bytes = cam.into_bytes();

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: cam_bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });

        uniform_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(&cam_bytes);
        uniform_buffer.unmap();

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("bind_group_layout"),
            });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0u64,
                    size: None,
                }),
            }],
            label: Some("bind_group"),
        });

        let vert_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<visual::ThickMeshVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
            ],
        };

        let inst_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![
                2 => Float32x4,
                3 => Float32x4,
                4 => Float32x4,
                5 => Float32x3,
            ],
        };

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&bind_group_layout],
                label: None,
                push_constant_ranges: &[],
            });

        let render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    buffers: &[vert_layout, inst_layout],
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                label: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: Default::default(),
            });

        let surf_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surf_config);

        let depth_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let depth_tex_desc = wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: depth_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let depth_texture = device.create_texture(&depth_tex_desc);

        Ok(Renderer {
            cam,
            res_mgr: mgr_builder.build(3000, &device),
            queue,
            device,
            surface_config: surf_config,
            surface,
            depth_texture,
            uniforms: uniform_buffer,
            pipeline: render_pipeline,
            bind_group,
            window_dimensions: (width, height),
        })
    }

    pub fn render<'r, 'i>(
        &'r mut self,
        dimensions @ (width, height): (u32, u32),
        instances: impl Iterator<Item = &'i dyn visual::Instance> + 'i,
    ) where
        'r: 'i,
    {
        if dimensions != self.window_dimensions {
            {
                let cfg = &mut self.surface_config;
                cfg.width = width;
                cfg.height = height;
            }

            self.surface.configure(&self.device, &self.surface_config);

            let depth_extent = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            let depth_tex_desc = wgpu::TextureDescriptor {
                label: Some("depth_texture"),
                size: depth_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            self.depth_texture = self.device.create_texture(&depth_tex_desc);

            self.window_dimensions = dimensions;
        }

        self.cam.w_h_ratio = width as f32 / height as f32;
        let cam_bytes = self.cam.into_bytes();

        self.queue.write_buffer(&self.uniforms, 0u64, &cam_bytes);
        let ranges = self.res_mgr.update(&self.queue, instances);

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                panic!("Failed to get current surface texture! Reason: {}", err)
            }
        };

        let out_color = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture_view =
            self.depth_texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("command_encoder"),
            },
        );

        {
            let mut rpass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view: &out_color,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        },
                    )],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_texture_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1f32),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        },
                    ),
                    label: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            let indices = self.res_mgr.indices();
            let vertices = self.res_mgr.vertices();
            let instances = self.res_mgr.instances();

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass
                .set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            rpass.set_vertex_buffer(0, vertices.slice(..));
            rpass.set_vertex_buffer(1, instances.slice(..));

            for (idx_range, vtx_range) in ranges {
                rpass.draw_indexed(idx_range, 0, vtx_range);
            }
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
    }
}
