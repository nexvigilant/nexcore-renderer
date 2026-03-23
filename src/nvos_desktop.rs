//! NVOS Desktop — bridges the NexCore OS compositor to the GPU renderer.
//!
//! Boots the Shell (compositor + layout + apps), creates a winit window
//! with the Vello GPU pipeline, and each frame:
//!   shell.tick() → framebuffer → BlitRgba → Vello → GPU → screen
//!
//! This closes Gap G1 (compositor → renderer bridge) and Gap G2
//! (nexcore-init has no renderer dependency).

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use nexcore_renderer::gpu::backend::RenderBackend;
use nexcore_renderer::gpu::vello_renderer::VelloRenderer;
use nexcore_renderer::paint::DisplayCommand;
use nexcore_renderer::style::Color;
use nexcore_renderer::text::TextRenderer;

use nexcore_pal::FormFactor;
use nexcore_shell::Shell;

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// NVOS Desktop application state.
struct NvosApp {
    window: Option<Arc<Window>>,
    renderer: Option<RenderBackend>,
    text_renderer: TextRenderer,
    shell: Shell,
}

impl NvosApp {
    fn new() -> Self {
        let mut shell = Shell::new(FormFactor::Desktop);
        shell.boot();
        tracing::info!("Shell booted — form factor: Desktop");

        Self {
            window: None,
            renderer: None,
            text_renderer: TextRenderer::new(),
            shell,
        }
    }

    fn init_window(&mut self, window: Window) {
        let window = Arc::new(window);
        self.window = Some(Arc::clone(&window));
        match pollster::block_on(VelloRenderer::new(window)) {
            Ok(r) => {
                self.renderer = Some(RenderBackend::Vello(r));
                tracing::info!("GPU renderer initialized");
            }
            Err(e) => tracing::error!("GPU init failed: {e}"),
        }
    }

    fn render_frame(&mut self) {
        let renderer = match self.renderer.as_mut() {
            Some(r) => r,
            None => return,
        };

        // Tick the OS shell — composites all surfaces
        self.shell.tick();

        // Get the composited RGBA framebuffer
        let fb = self.shell.framebuffer();
        let layout = self.shell.layout();
        let (fb_w, fb_h) = (layout.width, layout.height);

        // Get viewport size from the GPU renderer
        let (vw, vh) = renderer.size();

        // Build a single BlitRgba command that uploads the compositor output
        let commands = vec![DisplayCommand::BlitRgba {
            rect: nexcore_renderer::layout::Rect {
                x: 0.0,
                y: 0.0,
                width: vw as f32,
                height: vh as f32,
            },
            width: fb_w,
            height: fb_h,
            data: fb.to_vec(),
        }];

        let bg = Color {
            r: 13,
            g: 13,
            b: 20,
            a: 255,
        };

        if let Err(e) = renderer.render(&commands, bg, &mut self.text_renderer) {
            tracing::error!("Render failed: {e}");
        }
    }
}

impl ApplicationHandler for NvosApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("NVOS Desktop — NexVigilant Operating System")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        match event_loop.create_window(attrs) {
            Ok(window) => self.init_window(window),
            Err(e) => tracing::error!("Window creation failed: {e}"),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }
            }
            WindowEvent::RedrawRequested => self.render_frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting NVOS Desktop");

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            tracing::error!("Failed to create event loop: {e}");
            std::process::exit(1);
        }
    };

    let mut app = NvosApp::new();
    if let Err(e) = event_loop.run_app(&mut app) {
        tracing::error!("Event loop error: {e}");
        std::process::exit(1);
    }
}
