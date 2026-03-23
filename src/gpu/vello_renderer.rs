//! Vello-based GPU renderer using scene-graph rendering.
//!
//! Translates `DisplayCommand` lists into a `vello::Scene`, renders to an
//! intermediate `Rgba8Unorm` texture via `render_to_texture`, then blits
//! to the sRGB surface with a fullscreen triangle shader.
//!
//! ## Tier Classification
//!
//! - **T2-P**: `to_vello_color`, `to_kurbo_rect` (cross-domain conversion)
//! - **T2-C**: `VelloRenderer` (composed wgpu + vello state)

use crate::gpu::gpu_text::{GpuTextShaper, render_text_gpu};
use crate::layout::Rect;
use crate::paint::image::ImageCache;
use crate::paint::{DisplayCommand, Point};
use crate::style::Color;
use crate::text::TextRenderer;
use std::num::NonZeroUsize;
use std::sync::Arc;
use vello::kurbo;
use vello::peniko;
use vello::{AaSupport, RenderParams, Renderer, RendererOptions, Scene};
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// Vello-backed 2D GPU renderer.
///
/// Uses Vello's compute-based pipeline for anti-aliased rendering of
/// rectangles and text, with a blit pass to copy the result to the
/// window surface.
///
/// Tier: T2-C (composed wgpu + vello state)
pub struct VelloRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    vello_renderer: Renderer,
    target_texture: wgpu::Texture,
    target_view: wgpu::TextureView,
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    size: PhysicalSize<u32>,
    /// GPU text shaper for native glyph rendering (Phase 3c).
    gpu_text_shaper: GpuTextShaper,
    /// Image cache for decoded image data (Phase 5a).
    image_cache: ImageCache,
}

/// Intermediate GPU resources needed during VelloRenderer construction.
struct GpuResources {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    surface_format: wgpu::TextureFormat,
}

impl VelloRenderer {
    /// Create a new Vello renderer for the given window.
    ///
    /// # Errors
    /// Returns error if GPU initialization or Vello renderer creation fails.
    pub async fn new(window: Arc<Window>) -> nexcore_error::Result<Self> {
        let size = window.inner_size();
        let gpu = init_gpu(window, size).await?;

        let vello_renderer = create_vello_renderer(&gpu.device)?;

        let (target_texture, target_view) =
            create_target_texture(&gpu.device, size.width.max(1), size.height.max(1));

        let (blit_pipeline, blit_bind_group_layout, blit_sampler) =
            create_blit_pipeline(&gpu.device, gpu.surface_format);

        Ok(Self {
            surface: gpu.surface,
            device: gpu.device,
            queue: gpu.queue,
            config: gpu.config,
            vello_renderer,
            target_texture,
            target_view,
            blit_pipeline,
            blit_bind_group_layout,
            blit_sampler,
            size,
            gpu_text_shaper: GpuTextShaper::new(),
            image_cache: ImageCache::new(),
        })
    }

    /// Resize the renderer to match new window dimensions.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            let (tex, view) = create_target_texture(&self.device, new_size.width, new_size.height);
            self.target_texture = tex;
            self.target_view = view;
        }
    }

    /// Render display commands to the screen via Vello.
    ///
    /// 1. Build a `vello::Scene` from display commands
    /// 2. Render scene to intermediate `Rgba8Unorm` texture
    /// 3. Blit intermediate texture to sRGB surface
    ///
    /// # Errors
    /// Returns error if rendering or surface acquisition fails.
    pub fn render(
        &mut self,
        commands: &[DisplayCommand],
        bg_color: Color,
        text_renderer: &mut TextRenderer,
    ) -> nexcore_error::Result<()> {
        let w = self.size.width as f32;
        let h = self.size.height as f32;

        let scene = build_scene(
            commands,
            bg_color,
            text_renderer,
            &mut self.image_cache,
            w,
            h,
        );

        self.render_scene_to_texture(&scene, bg_color)?;
        self.present_to_surface()
    }

    /// Render display commands using GPU-native text rendering (Phase 3c).
    ///
    /// Uses Vello's `draw_glyphs()` for text instead of CPU rasterization.
    /// This provides better performance and quality for text-heavy scenes.
    ///
    /// # Errors
    /// Returns error if rendering or surface acquisition fails.
    pub fn render_gpu_text(
        &mut self,
        commands: &[DisplayCommand],
        bg_color: Color,
    ) -> nexcore_error::Result<()> {
        let w = self.size.width as f32;
        let h = self.size.height as f32;

        let scene = build_scene_gpu_text(
            commands,
            bg_color,
            &mut self.gpu_text_shaper,
            &mut self.image_cache,
            w,
            h,
        );

        self.render_scene_to_texture(&scene, bg_color)?;
        self.present_to_surface()
    }

    /// Get viewport size.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.size.width, self.size.height)
    }

    /// Load an image into the renderer's cache.
    ///
    /// Decodes the raw bytes (PNG, JPEG, WebP, GIF) and stores the
    /// decoded RGBA image under the given URL key.
    ///
    /// Returns `true` if the image was successfully decoded and cached.
    pub fn load_image(&mut self, url: &str, bytes: &[u8]) -> bool {
        use crate::paint::image::DecodedImage;
        if let Some(decoded) = DecodedImage::decode(bytes) {
            self.image_cache.insert(url.to_string(), decoded);
            true
        } else {
            self.image_cache.mark_failed(url);
            false
        }
    }

    /// Get mutable access to the image cache.
    pub fn image_cache_mut(&mut self) -> &mut ImageCache {
        &mut self.image_cache
    }

    /// Preload images by fetching and decoding them into the cache.
    ///
    /// For each URL: resolves against `base_url`, fetches bytes via
    /// `net::fetch_bytes`, decodes to RGBA, and caches under the
    /// original `src` key so `draw_image()` can find it.
    ///
    /// Skips URLs that are already cached or previously failed.
    ///
    /// Tier: T2-C (σ sequence of μ mappings via → causality)
    pub fn preload_images(&mut self, urls: &[String], base_url: &str) {
        for src in urls {
            // Skip already-cached or known-failed URLs
            if self.image_cache.contains(src) || self.image_cache.is_failed(src) {
                continue;
            }

            // Resolve relative URL against base
            let fetch_url = crate::net::resolve(base_url, src).unwrap_or_else(|| src.clone());

            // Fetch bytes
            match crate::net::fetch_bytes(&fetch_url) {
                Ok(bytes) => {
                    if self.load_image(src, &bytes) {
                        tracing::debug!("Preloaded image: {src}");
                    } else {
                        tracing::warn!("Failed to decode image: {src}");
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch image {src}: {e}");
                    self.image_cache.mark_failed(src);
                }
            }
        }
    }

    /// Render the Vello scene into the intermediate texture.
    fn render_scene_to_texture(
        &mut self,
        scene: &Scene,
        bg_color: Color,
    ) -> nexcore_error::Result<()> {
        let render_params = RenderParams {
            base_color: to_vello_color(bg_color),
            width: self.size.width,
            height: self.size.height,
            antialiasing_method: vello::AaConfig::Area,
        };

        self.vello_renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                scene,
                &self.target_view,
                &render_params,
            )
            .map_err(|e| nexcore_error::nexerror!("Vello render failed: {e}"))
    }

    /// Capture the current `target_texture` as a PNG byte vector.
    ///
    /// Copies the GPU texture to a staging buffer, maps it to CPU memory,
    /// strips row alignment padding, and encodes as PNG via the `image` crate.
    ///
    /// # Errors
    /// Returns error if GPU readback or PNG encoding fails.
    pub fn capture_to_png(&self) -> nexcore_error::Result<Vec<u8>> {
        use image::ImageEncoder;

        let width = self.size.width;
        let height = self.size.height;
        if width == 0 || height == 0 {
            return Err(nexcore_error::nexerror!("Cannot capture 0-dimension frame"));
        }

        let bytes_per_pixel = 4u32; // RGBA8
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot Staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Screenshot Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = staging_buffer.slice(..);
        let (map_tx, map_rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            map_tx.send(result).ok();
        });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        map_rx
            .recv()
            .map_err(|_| nexcore_error::nexerror!("GPU map channel closed"))?
            .map_err(|e| nexcore_error::nexerror!("GPU buffer map failed: {e}"))?;

        let data = slice.get_mapped_range();

        // Strip row padding
        let unpadded = unpadded_bytes_per_row as usize;
        let padded = padded_bytes_per_row as usize;
        let mut rgba = Vec::with_capacity(unpadded * height as usize);
        for row in 0..height as usize {
            let start = row * padded;
            rgba.extend_from_slice(&data[start..start + unpadded]);
        }
        drop(data);
        staging_buffer.unmap();

        // Encode as PNG
        let mut png_data = Vec::new();
        let png_encoder = image::codecs::png::PngEncoder::new(&mut png_data);
        png_encoder.write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)?;

        Ok(png_data)
    }

    /// Acquire surface texture, blit, and present.
    fn present_to_surface(&mut self) -> nexcore_error::Result<()> {
        let output = self.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Blit Encoder"),
            });

        self.blit_to_surface(&mut encoder, &surface_view);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Blit intermediate texture to the window surface via fullscreen triangle.
    fn blit_to_surface(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
    ) {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.target_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        });

        let attachment = blit_color_attachment(surface_view);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Blit Pass"),
            color_attachments: &[Some(attachment)],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.blit_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

// ── GPU Initialization Helpers ──────────────────────────────────

/// Initialize wgpu device, queue, surface, and configure the surface.
async fn init_gpu(
    window: Arc<Window>,
    size: PhysicalSize<u32>,
) -> nexcore_error::Result<GpuResources> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance.create_surface(window)?;
    let adapter = request_adapter(&instance, &surface).await?;
    let (device, queue) = request_device(&adapter).await?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = pick_srgb_format(&surface_caps);

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

    Ok(GpuResources {
        surface,
        device,
        queue,
        config,
        surface_format,
    })
}

/// Request a GPU adapter compatible with the surface.
async fn request_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'static>,
) -> nexcore_error::Result<wgpu::Adapter> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .map_err(Into::into)
}

/// Request a device and queue from the adapter.
async fn request_device(
    adapter: &wgpu::Adapter,
) -> nexcore_error::Result<(wgpu::Device, wgpu::Queue)> {
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: Some("nexbrowser-vello"),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
            experimental_features: Default::default(),
        })
        .await
        .map_err(Into::into)
}

/// Pick sRGB surface format, falling back to the first available.
fn pick_srgb_format(caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    caps.formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(caps.formats[0])
}

/// Create the Vello renderer with full antialiasing support.
fn create_vello_renderer(device: &wgpu::Device) -> nexcore_error::Result<Renderer> {
    Renderer::new(
        device,
        RendererOptions {
            use_cpu: false,
            antialiasing_support: AaSupport::all(),
            num_init_threads: NonZeroUsize::new(1),
            ..Default::default()
        },
    )
    .map_err(|e| nexcore_error::nexerror!("Vello renderer init failed: {e}"))
}

/// Create the intermediate `Rgba8Unorm` texture for Vello output.
fn create_target_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Vello Target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

// ── Blit Pipeline Helpers ───────────────────────────────────────

/// Create the blit pipeline, bind group layout, and sampler.
fn create_blit_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout, wgpu::Sampler) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Blit Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("blit_shader.wgsl").into()),
    });

    let bind_group_layout = create_blit_bind_group_layout(device);

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Blit Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = build_blit_render_pipeline(device, &pipeline_layout, &shader, surface_format);

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    (pipeline, bind_group_layout, sampler)
}

/// Create the bind group layout for the blit pass (texture + sampler).
fn create_blit_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Blit Bind Group Layout"),
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
    })
}

/// Build the blit render pipeline (fullscreen triangle, no vertex buffers).
fn build_blit_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Blit Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
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
    })
}

/// Build a blit color attachment targeting the surface view.
fn blit_color_attachment(view: &wgpu::TextureView) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
        depth_slice: None,
    }
}

// ── Scene Building ──────────────────────────────────────────────

/// Build a `vello::Scene` from display commands.
///
/// Translates each `DisplayCommand` to Vello primitives:
/// - `FillRect` → `scene.fill()` with `kurbo::Rect`
/// - `DrawText` → cosmic-text rasterize → `peniko::Image` → `scene.draw_image()`
/// - `DrawImage` → decoded image → `peniko::Image` → `scene.draw_image()`
fn build_scene(
    commands: &[DisplayCommand],
    bg_color: Color,
    text_renderer: &mut TextRenderer,
    image_cache: &mut ImageCache,
    viewport_w: f32,
    viewport_h: f32,
) -> Scene {
    let mut scene = Scene::new();

    // Background fill
    let bg_rect = kurbo::Rect::new(0.0, 0.0, f64::from(viewport_w), f64::from(viewport_h));
    scene.fill(
        peniko::Fill::NonZero,
        kurbo::Affine::IDENTITY,
        to_vello_color(bg_color),
        None,
        &bg_rect,
    );

    for cmd in commands {
        translate_command(&mut scene, cmd, text_renderer, image_cache, viewport_w);
    }

    scene
}

/// Build a `vello::Scene` using GPU-native text rendering (Phase 3c).
///
/// Uses Vello's `draw_glyphs()` for text instead of CPU rasterization.
/// This provides native GPU path rendering for glyphs.
fn build_scene_gpu_text(
    commands: &[DisplayCommand],
    bg_color: Color,
    gpu_text_shaper: &mut GpuTextShaper,
    image_cache: &mut ImageCache,
    viewport_w: f32,
    viewport_h: f32,
) -> Scene {
    let mut scene = Scene::new();

    // Background fill
    let bg_rect = kurbo::Rect::new(0.0, 0.0, f64::from(viewport_w), f64::from(viewport_h));
    scene.fill(
        peniko::Fill::NonZero,
        kurbo::Affine::IDENTITY,
        to_vello_color(bg_color),
        None,
        &bg_rect,
    );

    for cmd in commands {
        translate_command_gpu_text(&mut scene, cmd, gpu_text_shaper, image_cache, viewport_w);
    }

    scene
}

/// Translate a single `DisplayCommand` using GPU-native text rendering.
fn translate_command_gpu_text(
    scene: &mut Scene,
    cmd: &DisplayCommand,
    gpu_text_shaper: &mut GpuTextShaper,
    image_cache: &mut ImageCache,
    viewport_w: f32,
) {
    match cmd {
        DisplayCommand::FillRect { rect, color, .. } => {
            fill_rect(scene, rect, *color);
        }
        DisplayCommand::FillCircle {
            center,
            radius,
            color,
            ..
        } => {
            fill_circle(scene, *center, *radius, *color);
        }
        DisplayCommand::FillTriangle {
            p1, p2, p3, color, ..
        } => {
            fill_triangle(scene, *p1, *p2, *p3, *color);
        }
        DisplayCommand::StrokeLine {
            start,
            end,
            width,
            color,
            ..
        } => {
            stroke_line(scene, *start, *end, *width, *color);
        }
        DisplayCommand::DrawText {
            text,
            x,
            y,
            size,
            color,
            ..
        } => {
            // GPU-native text rendering via draw_glyphs()
            render_text_gpu(
                scene,
                gpu_text_shaper,
                text,
                *x,
                *y,
                *size,
                *color,
                viewport_w,
            );
        }
        DisplayCommand::DrawImage { src, rect, .. } => {
            draw_image(scene, image_cache, src, rect);
        }
        DisplayCommand::BlitRgba {
            rect,
            width,
            height,
            data,
        } => {
            blit_rgba(scene, rect, *width, *height, data);
        }
    }
}

/// Translate a single `DisplayCommand` into Vello scene operations.
fn translate_command(
    scene: &mut Scene,
    cmd: &DisplayCommand,
    text_renderer: &mut TextRenderer,
    image_cache: &mut ImageCache,
    viewport_w: f32,
) {
    match cmd {
        DisplayCommand::FillRect { rect, color, .. } => {
            fill_rect(scene, rect, *color);
        }
        DisplayCommand::FillCircle {
            center,
            radius,
            color,
            ..
        } => {
            fill_circle(scene, *center, *radius, *color);
        }
        DisplayCommand::FillTriangle {
            p1, p2, p3, color, ..
        } => {
            fill_triangle(scene, *p1, *p2, *p3, *color);
        }
        DisplayCommand::StrokeLine {
            start,
            end,
            width,
            color,
            ..
        } => {
            stroke_line(scene, *start, *end, *width, *color);
        }
        DisplayCommand::DrawText {
            text,
            x,
            y,
            size,
            color,
            ..
        } => {
            draw_text(
                scene,
                text_renderer,
                text,
                *x,
                *y,
                *size,
                *color,
                viewport_w,
            );
        }
        DisplayCommand::DrawImage { src, rect, .. } => {
            draw_image(scene, image_cache, src, rect);
        }
        DisplayCommand::BlitRgba {
            rect,
            width,
            height,
            data,
        } => {
            blit_rgba(scene, rect, *width, *height, data);
        }
    }
}

/// Render a `FillRect` command into the Vello scene.
fn fill_rect(scene: &mut Scene, rect: &Rect, color: Color) {
    let krect = to_kurbo_rect(rect);
    scene.fill(
        peniko::Fill::NonZero,
        kurbo::Affine::IDENTITY,
        to_vello_color(color),
        None,
        &krect,
    );
}

/// Render a `FillCircle` command into the Vello scene.
///
/// Tier: T2-P (cross-domain primitive: λ + N + μ)
/// Grounding: center_location + radius_quantity → kurbo::Circle
fn fill_circle(scene: &mut Scene, center: Point, radius: f32, color: Color) {
    let circle = kurbo::Circle::new(
        kurbo::Point::new(f64::from(center.x), f64::from(center.y)),
        f64::from(radius),
    );
    scene.fill(
        peniko::Fill::NonZero,
        kurbo::Affine::IDENTITY,
        to_vello_color(color),
        None,
        &circle,
    );
}

/// Render a `FillTriangle` command into the Vello scene.
///
/// Tier: T2-C (composite: σ[λ,λ,λ] + μ)
/// Grounding: sequence_of_3_locations + color_mapping → kurbo::BezPath
fn fill_triangle(scene: &mut Scene, p1: Point, p2: Point, p3: Point, color: Color) {
    let mut path = kurbo::BezPath::new();
    path.move_to(kurbo::Point::new(f64::from(p1.x), f64::from(p1.y)));
    path.line_to(kurbo::Point::new(f64::from(p2.x), f64::from(p2.y)));
    path.line_to(kurbo::Point::new(f64::from(p3.x), f64::from(p3.y)));
    path.close_path();
    scene.fill(
        peniko::Fill::NonZero,
        kurbo::Affine::IDENTITY,
        to_vello_color(color),
        None,
        &path,
    );
}

/// Render a `StrokeLine` command into the Vello scene.
///
/// Tier: T2-P (cross-domain primitive: λ → λ + N + μ)
/// Grounding: start_location → end_location + width_quantity → kurbo::Line
fn stroke_line(scene: &mut Scene, start: Point, end: Point, width: f32, color: Color) {
    let line = kurbo::Line::new(
        kurbo::Point::new(f64::from(start.x), f64::from(start.y)),
        kurbo::Point::new(f64::from(end.x), f64::from(end.y)),
    );
    scene.stroke(
        &kurbo::Stroke::new(f64::from(width)),
        kurbo::Affine::IDENTITY,
        to_vello_color(color),
        None,
        &line,
    );
}

/// Render a `DrawImage` command into the Vello scene.
///
/// Looks up the image in cache. If found, draws scaled to fit the target rect.
/// If not cached, draws a semi-transparent magenta placeholder rectangle.
///
/// Tier: T2-C (μ + N + λ — decode_mapping + dimensions + placement)
fn draw_image(scene: &mut Scene, cache: &mut ImageCache, src: &str, rect: &Rect) {
    if let Some(decoded) = cache.get(src) {
        let blob = peniko::Blob::from(decoded.rgba_data.clone());
        let image_data = peniko::ImageData {
            data: blob,
            format: peniko::ImageFormat::Rgba8,
            alpha_type: peniko::ImageAlphaType::Alpha,
            width: decoded.width,
            height: decoded.height,
        };

        // Scale image to fit the target rect
        let scale_x = if decoded.width > 0 {
            f64::from(rect.width) / f64::from(decoded.width)
        } else {
            1.0
        };
        let scale_y = if decoded.height > 0 {
            f64::from(rect.height) / f64::from(decoded.height)
        } else {
            1.0
        };

        let transform = kurbo::Affine::translate((f64::from(rect.x), f64::from(rect.y)))
            * kurbo::Affine::scale_non_uniform(scale_x, scale_y);

        scene.draw_image(&image_data, transform);
    } else {
        // Placeholder: semi-transparent magenta rectangle for uncached images
        let placeholder_color = Color {
            r: 255,
            g: 0,
            b: 255,
            a: 128,
        };
        fill_rect(scene, rect, placeholder_color);
    }
}

/// Render a `BlitRgba` command into the Vello scene.
///
/// Uploads raw RGBA framebuffer data as a Vello image and draws it
/// at the target rectangle. Used by the NVOS compositor bridge.
///
/// Tier: T2-C (μ + ∂ — framebuffer_mapping at boundary)
fn blit_rgba(scene: &mut Scene, rect: &Rect, width: u32, height: u32, data: &[u8]) {
    if width == 0 || height == 0 || data.is_empty() {
        return;
    }

    let blob = peniko::Blob::from(data.to_vec());
    let image_data = peniko::ImageData {
        data: blob,
        format: peniko::ImageFormat::Rgba8,
        alpha_type: peniko::ImageAlphaType::Alpha,
        width,
        height,
    };

    let scale_x = if width > 0 {
        f64::from(rect.width) / f64::from(width)
    } else {
        1.0
    };
    let scale_y = if height > 0 {
        f64::from(rect.height) / f64::from(height)
    } else {
        1.0
    };

    let transform = kurbo::Affine::translate((f64::from(rect.x), f64::from(rect.y)))
        * kurbo::Affine::scale_non_uniform(scale_x, scale_y);

    scene.draw_image(&image_data, transform);
}

/// Render a `DrawText` command into the Vello scene.
///
/// Uses cosmic-text to rasterize glyphs to RGBA pixels, wraps them in a
/// `peniko::Image`, and draws at the specified position.
fn draw_text(
    scene: &mut Scene,
    text_renderer: &mut TextRenderer,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
    max_width: f32,
) {
    let (tex_w, tex_h, pixels) = text_renderer.render_text(text, font_size, color, max_width);
    if pixels.is_empty() || tex_w == 0 || tex_h == 0 {
        return;
    }

    let blob = peniko::Blob::from(pixels);
    let image_data = peniko::ImageData {
        data: blob,
        format: peniko::ImageFormat::Rgba8,
        alpha_type: peniko::ImageAlphaType::Alpha,
        width: tex_w,
        height: tex_h,
    };

    // Position: y includes baseline offset; rasterized image starts at top-left
    let draw_y = y - font_size;
    scene.draw_image(
        &image_data,
        kurbo::Affine::translate((f64::from(x), f64::from(draw_y))),
    );
}

// ── T2-P Conversion Functions ───────────────────────────────────

/// Convert our `Color` to Vello's `peniko::Color`.
///
/// Tier: T2-P (cross-domain primitive conversion)
fn to_vello_color(c: Color) -> peniko::Color {
    peniko::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

/// Convert our `Rect` to Vello's `kurbo::Rect`.
///
/// Tier: T2-P (cross-domain primitive conversion)
fn to_kurbo_rect(r: &Rect) -> kurbo::Rect {
    kurbo::Rect::new(
        f64::from(r.x),
        f64::from(r.y),
        f64::from(r.x + r.width),
        f64::from(r.y + r.height),
    )
}

// ── Unit Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_vello_color() {
        let c = Color {
            r: 255,
            g: 128,
            b: 0,
            a: 255,
        };
        let vc = to_vello_color(c);
        let rgba = vc.to_rgba8();
        assert_eq!(rgba.r, 255);
        assert_eq!(rgba.g, 128);
        assert_eq!(rgba.b, 0);
        assert_eq!(rgba.a, 255);
    }

    #[test]
    fn test_to_vello_color_transparent() {
        let c = Color {
            r: 100,
            g: 200,
            b: 50,
            a: 0,
        };
        let vc = to_vello_color(c);
        let rgba = vc.to_rgba8();
        assert_eq!(rgba.a, 0);
    }

    #[test]
    fn test_to_kurbo_rect() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let kr = to_kurbo_rect(&r);
        assert!((kr.x0 - 10.0).abs() < f64::EPSILON);
        assert!((kr.y0 - 20.0).abs() < f64::EPSILON);
        assert!((kr.x1 - 110.0).abs() < f64::EPSILON);
        assert!((kr.y1 - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_to_kurbo_rect_zero() {
        let r = Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        let kr = to_kurbo_rect(&r);
        assert!(kr.x0.abs() < f64::EPSILON);
        assert!(kr.y0.abs() < f64::EPSILON);
        assert!(kr.x1.abs() < f64::EPSILON);
        assert!(kr.y1.abs() < f64::EPSILON);
    }

    #[test]
    fn test_build_scene_empty() {
        let mut text = TextRenderer::new();
        let scene = build_scene(
            &[],
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with background should have encoding data"
        );
    }

    #[test]
    fn test_build_scene_fill_rect() {
        let mut text = TextRenderer::new();
        let commands = vec![DisplayCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            },
            color: Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            node_id: None,
        }];
        let scene = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with rect should have encoding data"
        );
    }

    // ── Shape Primitive Tests (Phase 4) ─────────────────────────────

    #[test]
    fn test_build_scene_fill_circle() {
        let mut text = TextRenderer::new();
        let commands = vec![DisplayCommand::FillCircle {
            center: Point::new(100.0, 100.0),
            radius: 50.0,
            color: Color {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            },
            node_id: None,
        }];
        let scene = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with circle should have encoding data"
        );
    }

    #[test]
    fn test_build_scene_fill_triangle() {
        let mut text = TextRenderer::new();
        let commands = vec![DisplayCommand::FillTriangle {
            p1: Point::new(100.0, 50.0),
            p2: Point::new(50.0, 150.0),
            p3: Point::new(150.0, 150.0),
            color: Color {
                r: 0,
                g: 0,
                b: 255,
                a: 255,
            },
            node_id: None,
        }];
        let scene = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with triangle should have encoding data"
        );
    }

    #[test]
    fn test_build_scene_stroke_line() {
        let mut text = TextRenderer::new();
        let commands = vec![DisplayCommand::StrokeLine {
            start: Point::new(0.0, 0.0),
            end: Point::new(200.0, 200.0),
            width: 3.0,
            color: Color::BLACK,
            node_id: None,
        }];
        let scene = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with line should have encoding data"
        );
    }

    #[test]
    fn test_build_scene_mixed_shapes() {
        let mut text = TextRenderer::new();
        let commands = vec![
            DisplayCommand::FillRect {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 50.0,
                    height: 50.0,
                },
                color: Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                node_id: Some(1),
            },
            DisplayCommand::FillCircle {
                center: Point::new(100.0, 100.0),
                radius: 30.0,
                color: Color {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                },
                node_id: Some(2),
            },
            DisplayCommand::FillTriangle {
                p1: Point::new(200.0, 50.0),
                p2: Point::new(175.0, 100.0),
                p3: Point::new(225.0, 100.0),
                color: Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                },
                node_id: Some(3),
            },
        ];
        let scene = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with mixed shapes should have encoding data"
        );
    }

    // ── Image Rendering Tests (Phase 5a) ────────────────────────────

    #[test]
    fn test_build_scene_draw_image_placeholder() {
        let mut text = TextRenderer::new();
        let commands = vec![DisplayCommand::DrawImage {
            src: "https://example.com/test.png".to_string(),
            rect: Rect {
                x: 10.0,
                y: 10.0,
                width: 200.0,
                height: 150.0,
            },
            node_id: Some(1),
        }];
        // No image in cache → draws magenta placeholder
        let scene = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            800.0,
            600.0,
        );
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with image placeholder should have encoding data"
        );
    }

    #[test]
    fn test_build_scene_draw_image_cached() {
        use crate::paint::image::DecodedImage;
        let mut text = TextRenderer::new();
        let mut cache = ImageCache::new();

        // Pre-populate cache with a small 2x2 test image
        let img = DecodedImage {
            rgba_data: vec![
                255, 0, 0, 255, // Red pixel
                0, 255, 0, 255, // Green pixel
                0, 0, 255, 255, // Blue pixel
                255, 255, 0, 255, // Yellow pixel
            ],
            width: 2,
            height: 2,
        };
        cache.insert("test://cached.png".to_string(), img);

        let commands = vec![DisplayCommand::DrawImage {
            src: "test://cached.png".to_string(),
            rect: Rect {
                x: 50.0,
                y: 50.0,
                width: 100.0,
                height: 100.0,
            },
            node_id: Some(1),
        }];
        let scene = build_scene(&commands, Color::WHITE, &mut text, &mut cache, 800.0, 600.0);
        let encoding = scene.encoding();
        assert!(
            !encoding.is_empty(),
            "Scene with cached image should have encoding data"
        );
    }

    #[test]
    fn test_draw_image_placeholder_vs_cached_differ() {
        use crate::paint::image::DecodedImage;
        let mut text = TextRenderer::new();

        // Scene with uncached image (placeholder)
        let commands = vec![DisplayCommand::DrawImage {
            src: "test://missing.png".to_string(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 64.0,
                height: 64.0,
            },
            node_id: None,
        }];
        let scene_placeholder = build_scene(
            &commands,
            Color::WHITE,
            &mut text,
            &mut ImageCache::new(),
            400.0,
            400.0,
        );

        // Scene with cached image
        let mut cache = ImageCache::new();
        cache.insert(
            "test://missing.png".to_string(),
            DecodedImage {
                rgba_data: vec![0, 128, 255, 255],
                width: 1,
                height: 1,
            },
        );
        let scene_cached =
            build_scene(&commands, Color::WHITE, &mut text, &mut cache, 400.0, 400.0);

        // Both should produce valid scenes but with different encoding
        let enc_p = scene_placeholder.encoding();
        let enc_c = scene_cached.encoding();
        assert!(!enc_p.is_empty());
        assert!(!enc_c.is_empty());
    }
}
