use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 950;
const MAX_ITER: u32 = 150;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    center_x: f32,
    center_y: f32,
    scale: f32,
    width: f32,
    height: f32,
    max_iter: u32,
    _pad0: u32,
    _pad1: u32,
}

struct Vista {
    center_x: f64,
    center_y: f64,
    scale: f64,
}

impl Default for Vista {
    fn default() -> Self {
        Self {
            center_x: -0.5,
            center_y: 0.0,
            scale: 3.0,
        }
    }
}

impl Vista {
    fn zoom_at(&mut self, px: f64, py: f64, view_w: f64, view_h: f64, factor: f64) {
        let aspect = view_w / view_h;

        let old_x_span = self.scale;
        let old_y_span = old_x_span / aspect;
        let old_left = self.center_x - old_x_span * 0.5;
        let old_top = self.center_y - old_y_span * 0.5;

        let re_cursor = old_left + (px / view_w) * old_x_span;
        let im_cursor = old_top + (py / view_h) * old_y_span;

        self.scale = (self.scale * factor).clamp(1e-6, 10.0);

        let new_x_span = self.scale;
        let new_y_span = new_x_span / aspect;

        let u = px / view_w;
        let v = py / view_h;

        let new_left = re_cursor - u * new_x_span;
        let new_top = im_cursor - v * new_y_span;

        self.center_x = new_left + new_x_span * 0.5;
        self.center_y = new_top + new_y_span * 0.5;
    }

    fn pan_pixels(&mut self, dx: f64, dy: f64, view_w: f64, view_h: f64) {
        let aspect = view_w / view_h;
        let x_span = self.scale;
        let y_span = x_span / aspect;

        self.center_x -= (dx / view_w) * x_span;
        self.center_y -= (dy / view_h) * y_span;
    }

    fn to_uniforms(&self, width: u32, height: u32) -> Uniforms {
        Uniforms {
            center_x: self.center_x as f32,
            center_y: self.center_y as f32,
            scale: self.scale as f32,
            width: width as f32,
            height: height as f32,
            max_iter: MAX_ITER,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Mandelbrot GPU (wgpu)")
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
            .build(&event_loop)?,
    );

    let (device, queue, surface, mut config, pipeline, uniform_buffer, bind_group) =
        pollster::block_on(async {
            let instance = wgpu::Instance::default();
            let surface = instance.create_surface(window.clone())?;

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                    apply_limit_buckets: Default::default(),
                })
                .await?;

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await?;

            let size = window.inner_size();
            let caps = surface.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(caps.formats[0]);

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
                color_space: wgpu::SurfaceColorSpace::Auto,
            };
            surface.configure(&device, &config);

            let shader_src = include_str!("shader.wgsl");
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Mandelbrot Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });

            let vista_init = Vista::default();
            let uniforms_init = vista_init.to_uniforms(config.width, config.height);

            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Uniform Buffer"),
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms_init));

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Uniform Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Uniform Bind Group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Mandelbrot Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

            Ok::<_, Box<dyn std::error::Error>>((
                device,
                queue,
                surface,
                config,
                pipeline,
                uniform_buffer,
                bind_group,
            ))
        })?;

    let mut vista = Vista::default();
    let mut cursor_pos = (WIDTH as f64 * 0.5, HEIGHT as f64 * 0.5);
    let mut dragging = false;
    let mut sporco = true;

    let window_clone = Arc::clone(&window);

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::CursorMoved { position, .. } => {
                    let nuova_pos = (position.x, position.y);
                    if dragging {
                        let dx = nuova_pos.0 - cursor_pos.0;
                        let dy = nuova_pos.1 - cursor_pos.1;
                        vista.pan_pixels(dx, dy, config.width as f64, config.height as f64);
                        sporco = true;
                    }
                    cursor_pos = nuova_pos;
                }

                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => {
                    dragging = state == ElementState::Pressed;
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let raw_y = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as f64,
                        MouseScrollDelta::PixelDelta(pos) => pos.y / 120.0,
                    };
                    let scroll_y = raw_y.clamp(-4.0, 4.0);

                    if scroll_y != 0.0 {
                        let factor = 0.85_f64.powf(scroll_y);
                        vista.zoom_at(
                            cursor_pos.0,
                            cursor_pos.1,
                            config.width as f64,
                            config.height as f64,
                            factor,
                        );
                        sporco = true;
                    }
                }

                WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        config.width = size.width;
                        config.height = size.height;
                        surface.configure(&device, &config);
                        sporco = true;
                        window_clone.request_redraw();
                    }
                }

                WindowEvent::RedrawRequested => {
                    if sporco {
                        let uniforms = vista.to_uniforms(config.width, config.height);
                        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
                        sporco = false;
                    }

                    let output = match surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(tex)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
                        wgpu::CurrentSurfaceTexture::Lost
                        | wgpu::CurrentSurfaceTexture::Outdated => {
                            surface.configure(&device, &config);
                            window_clone.request_redraw();
                            return;
                        }
                        wgpu::CurrentSurfaceTexture::Timeout
                        | wgpu::CurrentSurfaceTexture::Occluded => {
                            return;
                        }
                        wgpu::CurrentSurfaceTexture::Validation => {
                            eprintln!("Errore di validazione surface");
                            elwt.exit();
                            return;
                        }
                    };

                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Mandelbrot Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });

                        rpass.set_pipeline(&pipeline);
                        rpass.set_bind_group(0, &bind_group, &[]);
                        rpass.draw(0..3, 0..1);
                    }

                    queue.submit(std::iter::once(encoder.finish()));
                    queue.present(output);
                    window_clone.set_title(&format!("Mandelbrot GPU | zoom: {:.2e}", vista.scale));
                }
                _ => (),
            },

            Event::AboutToWait => {
                if sporco {
                    window_clone.request_redraw();
                }
            }
            _ => (),
        }
    })?;

    Ok(())
}
