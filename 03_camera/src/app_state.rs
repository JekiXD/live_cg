use std::sync::Arc;

use glam::Vec3;
use winit::window::Window;
use crate::{camera::Camera, uniform, vertex::{load_model, BufferGeometry, Vertex}};

pub struct AppState {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    uniform: uniform::Uniform,
    model: BufferGeometry
}

impl AppState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions{
            compatible_surface: Some(&surface),
            ..Default::default()
        }).await.unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default(), None).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            width: size.width,
            height: size.height,
            format: surface_caps.formats[0],
            alpha_mode: surface_caps.alpha_modes[0],
            present_mode: surface_caps.present_modes[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            desired_maximum_frame_latency: 2,
            view_formats: vec![]
        };
        surface.configure(&device, &config);

        let camera_pos = Vec3::new(0.0, 10.0, 10.0);
        let camera = Camera::new(
            70.0, 
            size.width as f32 / size.height as f32, 
            0.01, 
            1000.0, 
            camera_pos, 
            Vec3::new(0.0, 1.0, 0.0), 
            Vec3::new(0.0, 0.0, 0.0) - camera_pos
        );

        let uniform = uniform::Uniform::new(&device, camera);

        let model = load_model(&device);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into())
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &uniform.bind_group_layout
            ],
            ..Default::default()
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Vertex::desc()
                ]
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::all()
                    })
                ]
            }),
            primitive: wgpu::PrimitiveState { 
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false, 
                polygon_mode: wgpu::PolygonMode::Fill, 
                conservative: false 
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None
        });



        AppState {
            window,
            surface,
            device,
            queue,
            render_pipeline,
            uniform,
            model
        }
    }

    pub fn update(&mut self) {
        self.uniform.update(&self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let texture = self.surface.get_current_texture()?;
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(
                        wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations{
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store
                            }
                        }
                    )
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform.bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.model.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.model.indices.len() as u32, 0, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        texture.present();

        Ok(())
    }
}