//! GPU rendering module using wgpu.

pub mod backend;
pub mod gpu_text;
pub mod vello_renderer;

use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::style::Color;
use crate::text::TextRenderer;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// Vertex for GPU rendering (solid color).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Vertex for textured rendering (text).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexturedVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl TexturedVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TexturedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// GPU renderer state.
pub struct GpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    size: winit::dpi::PhysicalSize<u32>,
}

impl GpuRenderer {
    /// Create a new GPU renderer for a window.
    ///
    /// # Errors
    /// Returns error if GPU initialization fails.
    pub async fn new(window: Arc<Window>) -> nexcore_error::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: Some("nexbrowser"),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: Default::default(),
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Text rendering pipeline
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text_shader.wgsl").into()),
        });

        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Text Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&text_bind_group_layout],
            push_constant_ranges: &[],
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs_main"),
                buffers: &[TexturedVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            text_pipeline,
            text_bind_group_layout,
            sampler,
            size,
        })
    }

    /// Resize the renderer.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Build a clear-color attachment for the render pass.
    fn color_attachment<'a>(
        view: &'a wgpu::TextureView,
        bg: Color,
    ) -> wgpu::RenderPassColorAttachment<'a> {
        let clear = wgpu::Color {
            r: f64::from(bg.r) / 255.0,
            g: f64::from(bg.g) / 255.0,
            b: f64::from(bg.b) / 255.0,
            a: f64::from(bg.a) / 255.0,
        };
        wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        }
    }

    /// Record draw calls into a render pass.
    fn record_draws(
        render_pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        vertex_buffer: &wgpu::Buffer,
        vertex_count: u32,
        text_pipeline: &wgpu::RenderPipeline,
        text_data: &[(wgpu::BindGroup, wgpu::Buffer, u32)],
    ) {
        if vertex_count > 0 {
            render_pass.set_pipeline(pipeline);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..vertex_count, 0..1);
        }
        render_pass.set_pipeline(text_pipeline);
        for (bind_group, vertex_buf, count) in text_data {
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buf.slice(..));
            render_pass.draw(0..*count, 0..1);
        }
    }

    /// Render display commands to the screen.
    ///
    /// # Errors
    /// Returns error if rendering fails.
    pub fn render(
        &mut self,
        commands: &[DisplayCommand],
        bg_color: Color,
        text_renderer: &mut TextRenderer,
    ) -> nexcore_error::Result<()> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let vertices = self.build_vertices(commands);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let text_data = self.build_text_data(commands, text_renderer);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let attachment = Self::color_attachment(&view, bg_color);
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(attachment)],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            Self::record_draws(
                &mut render_pass,
                &self.pipeline,
                &vertex_buffer,
                vertices.len() as u32,
                &self.text_pipeline,
                &text_data,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn build_text_data(
        &self,
        commands: &[DisplayCommand],
        text_renderer: &mut TextRenderer,
    ) -> Vec<(wgpu::BindGroup, wgpu::Buffer, u32)> {
        let w = self.size.width as f32;
        let h = self.size.height as f32;
        let mut result = Vec::new();

        for cmd in commands {
            if let DisplayCommand::DrawText {
                text,
                x,
                y,
                size,
                color,
                ..
            } = cmd
            {
                let (tex_w, tex_h, pixels) = text_renderer.render_text(text, *size, *color, w);
                if pixels.is_empty() || tex_w == 0 || tex_h == 0 {
                    continue;
                }

                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Text Texture"),
                    size: wgpu::Extent3d {
                        width: tex_w,
                        height: tex_h,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });

                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * tex_w),
                        rows_per_image: Some(tex_h),
                    },
                    wgpu::Extent3d {
                        width: tex_w,
                        height: tex_h,
                        depth_or_array_layers: 1,
                    },
                );

                let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Text Bind Group"),
                    layout: &self.text_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });

                // Create quad vertices for text
                let x1 = (*x / w) * 2.0 - 1.0;
                let y1 = 1.0 - (*y / h) * 2.0;
                let x2 = ((*x + tex_w as f32) / w) * 2.0 - 1.0;
                let y2 = 1.0 - ((*y + tex_h as f32) / h) * 2.0;

                let vertices = [
                    TexturedVertex {
                        position: [x1, y1],
                        tex_coords: [0.0, 0.0],
                    },
                    TexturedVertex {
                        position: [x2, y1],
                        tex_coords: [1.0, 0.0],
                    },
                    TexturedVertex {
                        position: [x1, y2],
                        tex_coords: [0.0, 1.0],
                    },
                    TexturedVertex {
                        position: [x1, y2],
                        tex_coords: [0.0, 1.0],
                    },
                    TexturedVertex {
                        position: [x2, y1],
                        tex_coords: [1.0, 0.0],
                    },
                    TexturedVertex {
                        position: [x2, y2],
                        tex_coords: [1.0, 1.0],
                    },
                ];

                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Text Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                result.push((bind_group, vertex_buffer, 6));
            }
        }

        result
    }

    fn build_vertices(&self, commands: &[DisplayCommand]) -> Vec<Vertex> {
        let mut vertices = Vec::new();
        let w = self.size.width as f32;
        let h = self.size.height as f32;

        for cmd in commands {
            if let DisplayCommand::FillRect { rect, color, .. } = cmd {
                let verts = self.rect_to_vertices(rect, color, w, h);
                vertices.extend(verts);
            }
            // Text rendering handled separately via cosmic-text
        }

        vertices
    }

    fn rect_to_vertices(&self, rect: &Rect, color: &Color, w: f32, h: f32) -> [Vertex; 6] {
        // Convert to normalized device coordinates (-1 to 1)
        let x1 = (rect.x / w) * 2.0 - 1.0;
        let y1 = 1.0 - (rect.y / h) * 2.0;
        let x2 = ((rect.x + rect.width) / w) * 2.0 - 1.0;
        let y2 = 1.0 - ((rect.y + rect.height) / h) * 2.0;

        let c = [
            f32::from(color.r) / 255.0,
            f32::from(color.g) / 255.0,
            f32::from(color.b) / 255.0,
            f32::from(color.a) / 255.0,
        ];

        [
            Vertex {
                position: [x1, y1],
                color: c,
            },
            Vertex {
                position: [x2, y1],
                color: c,
            },
            Vertex {
                position: [x1, y2],
                color: c,
            },
            Vertex {
                position: [x1, y2],
                color: c,
            },
            Vertex {
                position: [x2, y1],
                color: c,
            },
            Vertex {
                position: [x2, y2],
                color: c,
            },
        ]
    }

    /// Get viewport size.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.size.width, self.size.height)
    }
}
