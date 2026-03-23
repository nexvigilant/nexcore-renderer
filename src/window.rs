//! Window management and event loop.
//!
//! Owns the winit event loop, GPU renderer, and dispatches input
//! events through `InputState` into `BrowserAction` values that
//! drive navigation, scrolling, and the address bar overlay.
//!
//! Now integrates `NexBrowserState` for GROUNDED loop, chrome widgets,
//! and sidebar panels alongside the existing `Browser` content renderer.

use crate::Browser;
use crate::bridge::NexCoreBridge;
use crate::chrome::sidebar::Sidebar;
use crate::chrome::status_bar::StatusBar;
use crate::chrome::tab_bar::{TabBar, TabInfo};
use crate::chrome::toolbar::Toolbar;
use crate::chrome::{ChromeLayout, Widget};
use crate::debug_server::FrameBridge;
use crate::gpu::backend::RenderBackend;
use crate::gpu::vello_renderer::VelloRenderer;
use crate::input::{BrowserAction, InputState};
use crate::paint::DisplayCommand;
use crate::panels::Panel;
use crate::panels::brain_viewer::{BrainViewerPanel, SessionDisplay};
use crate::panels::cloud_dashboard::{CloudDashboardPanel, CloudServiceDisplay};
use crate::panels::experience_store::ExperienceStorePanel;
use crate::panels::grounded_monitor::GroundedMonitor;
use crate::panels::guardian_monitor::{ActuatorDisplay, GuardianMonitorPanel, SensorDisplay};
use crate::panels::hypothesis_queue::HypothesisQueuePanel;
use crate::panels::mcp_explorer::{McpExplorerPanel, SkillDisplay};
use crate::panels::signal_dashboard::SignalDashboard;
use crate::scroll::{ScrollState, apply_scroll_transform, build_scrollbar_commands};
use crate::state::{BridgeResult, NexBrowserState, PanelId, TabId};
use crate::style::Color;
use crate::text::TextRenderer;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Application state for winit event loop.
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<RenderBackend>,
    text_renderer: TextRenderer,
    input_state: InputState,
    browser: Browser,
    url: String,
    nex_state: NexBrowserState,
    tab_bar: TabBar,
    toolbar: Toolbar,
    sidebar: Sidebar,
    status_bar: StatusBar,
    // ── Phase 1 panels ──────────────────────────────────────
    grounded_panel: GroundedMonitor,
    hypothesis_panel: HypothesisQueuePanel,
    experience_panel: ExperienceStorePanel,
    signal_panel: SignalDashboard,
    // ── Phase 2 panels ──────────────────────────────────────
    brain_panel: BrainViewerPanel,
    guardian_panel: GuardianMonitorPanel,
    mcp_panel: McpExplorerPanel,
    cloud_panel: CloudDashboardPanel,
    // ── Bridge ──────────────────────────────────────────────
    bridge: NexCoreBridge,
    // ── Debug server ────────────────────────────────────────
    frame_bridge: Option<Arc<FrameBridge>>,
    // ── Scroll ──────────────────────────────────────────────
    scroll_state: ScrollState,
}

impl App {
    fn new(url: String) -> Self {
        Self {
            window: None,
            renderer: None,
            text_renderer: TextRenderer::new(),
            input_state: InputState::new(),
            browser: Browser::new(),
            url,
            nex_state: NexBrowserState::new(),
            tab_bar: TabBar::new(),
            toolbar: Toolbar::new(),
            sidebar: Sidebar::new(),
            status_bar: StatusBar::new(),
            grounded_panel: GroundedMonitor::new(),
            hypothesis_panel: HypothesisQueuePanel::new(),
            experience_panel: ExperienceStorePanel::new(),
            signal_panel: SignalDashboard::new(),
            brain_panel: BrainViewerPanel::new(),
            guardian_panel: GuardianMonitorPanel::new(),
            mcp_panel: McpExplorerPanel::new(),
            cloud_panel: CloudDashboardPanel::default(),
            bridge: NexCoreBridge::new(),
            frame_bridge: crate::debug_server::start(),
            scroll_state: ScrollState::new(),
        }
    }

    fn viewport_size(&self) -> (f32, f32) {
        self.renderer.as_ref().map_or((1280.0, 720.0), |r| {
            let (w, h) = r.size();
            (w as f32, h as f32)
        })
    }
}

// ── ApplicationHandler ─────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("NexBrowser - GROUNDED AI Collaborator")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

        match event_loop.create_window(attrs) {
            Ok(window) => self.init_window(window),
            Err(e) => tracing::error!("Window creation failed: {e}"),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        dispatch_event(self, event_loop, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Dispatch a window event to the appropriate handler.
fn dispatch_event(app: &mut App, event_loop: &ActiveEventLoop, event: WindowEvent) {
    match event {
        WindowEvent::CloseRequested => event_loop.exit(),
        WindowEvent::Resized(size) => app.handle_resize(size),
        WindowEvent::RedrawRequested => app.render_frame(),
        WindowEvent::MouseInput { state, button, .. } => {
            let action = app.input_state.handle_mouse_button(button, state);
            app.handle_action(action);
        }
        WindowEvent::CursorMoved { position, .. } => {
            app.input_state
                .set_mouse_pos(position.x as f32, position.y as f32);
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let (dx, dy) = extract_scroll_delta(delta);
            let action = app.input_state.handle_scroll(dx, dy);
            app.handle_action(action);
        }
        WindowEvent::KeyboardInput { event, .. } => {
            let action = app.input_state.handle_key(&event);
            app.handle_action(action);
        }
        WindowEvent::ModifiersChanged(modifiers) => {
            app.input_state.set_modifiers(modifiers.state());
        }
        _ => {}
    }
}

/// Extract scroll delta from winit event.
fn extract_scroll_delta(delta: winit::event::MouseScrollDelta) -> (f32, f32) {
    match delta {
        winit::event::MouseScrollDelta::LineDelta(x, y) => (x * 20.0, y * 20.0),
        winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
    }
}

/// Extract background color from the active tab's layout.
fn extract_bg_color(browser: &Browser) -> Color {
    browser
        .active_tab()
        .and_then(|t| t.layout.as_ref())
        .map(|l| l.style.background_color)
        .unwrap_or(Color::WHITE)
}

/// Collect chrome widget display commands.
fn collect_chrome_commands(
    tab_bar: &TabBar,
    toolbar: &Toolbar,
    sidebar: &Sidebar,
    status_bar: &StatusBar,
) -> Vec<DisplayCommand> {
    let mut cmds = tab_bar.paint();
    cmds.extend(toolbar.paint());
    cmds.extend(sidebar.paint());
    cmds.extend(status_bar.paint());
    cmds
}

// ── App methods ────────────────────────────────────────────────

impl App {
    fn init_window(&mut self, window: Window) {
        let window = Arc::new(window);
        self.window = Some(Arc::clone(&window));
        match pollster::block_on(VelloRenderer::new(window)) {
            Ok(r) => {
                self.renderer = Some(RenderBackend::Vello(r));
                self.navigate_initial();
            }
            Err(e) => tracing::error!("GPU init failed: {e}"),
        }
    }

    fn navigate_initial(&mut self) {
        if let Err(e) = self.browser.navigate(&self.url) {
            tracing::error!("Navigation failed: {e}");
        }
        self.preload_page_images(&self.url.clone());
        self.input_state.address_bar.set_url(&self.url);
        self.toolbar.set_url(&self.url);
        self.sync_chrome();
    }

    fn handle_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size);
        }
        let layout = ChromeLayout::compute(
            size.width as f32,
            size.height as f32,
            self.nex_state.sidebar_visible,
        );
        self.browser
            .resize(layout.content.width, layout.content.height);
    }

    fn render_frame(&mut self) {
        if self.renderer.is_none() {
            return;
        }
        let bg = extract_bg_color(&self.browser);
        let (vw, vh) = self.viewport_size();
        let layout = ChromeLayout::compute(vw, vh, self.nex_state.sidebar_visible);

        // Layout chrome widgets
        self.tab_bar.layout(layout.tab_bar);
        self.toolbar.layout(layout.toolbar);
        self.sidebar.layout(layout.sidebar);
        self.status_bar.layout(layout.status_bar);

        // Build combined display list with scroll transform on content
        let bar_cmds = self.input_state.address_bar.build_display_commands(vw);
        let chrome_cmds = collect_chrome_commands(
            &self.tab_bar,
            &self.toolbar,
            &self.sidebar,
            &self.status_bar,
        );
        let panel_cmds = self.get_active_panel_cmds(layout.sidebar);
        let content = self.browser.display_list();

        // Update scroll state viewport from layout
        self.scroll_state
            .set_viewport(layout.content.width, layout.content.height);

        // Compute content height from layout tree
        if let Some(tab) = self.browser.active_tab() {
            if let Some(ref layout_box) = tab.layout {
                let content_h = layout_box.rect.y + layout_box.rect.height;
                self.scroll_state
                    .set_content_size(layout_box.rect.width, content_h);
            }
        }

        // Apply scroll transform to page content only (not chrome)
        let scrolled_content = apply_scroll_transform(content, &self.scroll_state);
        let scrollbar_cmds = build_scrollbar_commands(&self.scroll_state, 0.0);

        let total = scrolled_content.len()
            + chrome_cmds.len()
            + panel_cmds.len()
            + bar_cmds.len()
            + scrollbar_cmds.len();
        let mut combined = Vec::with_capacity(total);
        combined.extend(scrolled_content);
        combined.extend(chrome_cmds);
        combined.extend(panel_cmds);
        combined.extend(bar_cmds);
        combined.extend(scrollbar_cmds);

        if let Some(r) = self.renderer.as_mut() {
            if let Err(e) = r.render(&combined, bg, &mut self.text_renderer) {
                tracing::error!("Render failed: {e}");
            }
        }

        // Serve frame to debug server if requested
        if let Some(ref bridge) = self.frame_bridge {
            if bridge.check_requested() {
                if let Some(r) = self.renderer.as_ref() {
                    match r.capture_to_png() {
                        Ok(png) => bridge.deliver_frame(png),
                        Err(e) => {
                            tracing::warn!("Frame capture failed: {e}");
                            bridge.deliver_frame(Vec::new());
                        }
                    }
                }
            }
        }
    }

    fn sync_chrome(&mut self) {
        let title = self
            .browser
            .active_tab()
            .map_or("New Tab", |t| &t.title)
            .to_string();

        self.tab_bar.set_tabs(vec![TabInfo {
            id: TabId(0),
            title,
            active: true,
        }]);
        self.tab_bar
            .set_grounded_cycle(self.nex_state.grounded.cycle_count());
        self.toolbar.set_url(&self.url);
        self.sidebar.set_active(self.nex_state.active_panel);
        self.sidebar.set_visible(self.nex_state.sidebar_visible);
        self.status_bar.set_grounded_status(
            self.nex_state.grounded.cycle_count(),
            self.nex_state.grounded.confidence(),
            true,
            self.nex_state.grounded.learning_count(),
        );

        // ── Sync Phase 1 panels from GROUNDED loop ─────────────
        self.grounded_panel.sync(
            self.nex_state.grounded.cycle_count(),
            self.nex_state.grounded.confidence(),
            self.nex_state.grounded.active_claim(),
            self.nex_state.grounded.queue_len(),
            self.nex_state.grounded.learning_count(),
        );

        // ── Fetch Phase 2 data on panel switch ─────────────────
        self.fetch_active_panel_data();
    }

    /// Fetch data for the currently active Phase 2 panel.
    fn fetch_active_panel_data(&mut self) {
        match self.nex_state.active_panel {
            PanelId::BRAIN => self.fetch_brain_data(),
            PanelId::GUARDIAN => self.fetch_guardian_data(),
            PanelId::MCP => self.fetch_mcp_data(),
            PanelId::CLOUD => self.fetch_cloud_data(),
            _ => {} // Phase 1 panels sync from local state
        }
    }

    fn handle_action(&mut self, action: BrowserAction) {
        match action {
            BrowserAction::Navigate(url) => self.do_navigate(&url),
            BrowserAction::Reload => self.do_reload(),
            BrowserAction::Scroll { dx, dy } => {
                self.scroll_state.scroll_by(dx, dy);
                self.browser.scroll(dy);
            }
            BrowserAction::ScrollPage { down } => {
                if down {
                    self.scroll_state.page_down();
                } else {
                    self.scroll_state.page_up();
                }
            }
            BrowserAction::ScrollToEdge { top } => {
                if top {
                    self.scroll_state.scroll_to_top();
                } else {
                    self.scroll_state.scroll_to_bottom();
                }
            }
            BrowserAction::ScrollLine { down } => {
                if down {
                    self.scroll_state.scroll_line_down();
                } else {
                    self.scroll_state.scroll_line_up();
                }
            }
            BrowserAction::Zoom(factor) => self.browser.zoom(factor),
            BrowserAction::Click { x, y } => self.do_click(x, y),
            BrowserAction::FocusAddressBar => self.toolbar.set_focused(true),
            BrowserAction::DevTools => tracing::info!("DevTools (not implemented)"),
            BrowserAction::Back | BrowserAction::Forward => {
                tracing::info!("History (not implemented)");
            }
            _ => {}
        }
    }

    fn do_navigate(&mut self, url: &str) {
        tracing::info!("Navigating to: {url}");
        if let Err(e) = self.browser.navigate(url) {
            tracing::error!("Navigation failed: {e}");
            return;
        }
        self.preload_page_images(url);
        self.url = url.to_string();
        self.input_state.address_bar.set_url(&self.url);
        self.sync_chrome();
    }

    fn do_reload(&mut self) {
        let url = self.url.clone();
        if let Err(e) = self.browser.navigate(&url) {
            tracing::error!("Reload failed: {e}");
            return;
        }
        self.preload_page_images(&url);
    }

    fn do_click(&mut self, x: f32, y: f32) {
        if let Some(href) = self.browser.find_link_at(x, y) {
            self.do_navigate(&href);
        }
    }

    /// Scan the current display list for image URLs and preload them.
    fn preload_page_images(&mut self, base_url: &str) {
        let image_urls = crate::paint::collect_image_urls(self.browser.display_list());
        if !image_urls.is_empty() {
            tracing::info!("Preloading {} image(s)", image_urls.len());
            if let Some(renderer) = &mut self.renderer {
                renderer.preload_images(&image_urls, base_url);
            }
        }
    }

    // ── Panel rendering ─────────────────────────────────────────

    /// Get display commands for the currently active panel.
    fn get_active_panel_cmds(&self, area: crate::layout::Rect) -> Vec<DisplayCommand> {
        match self.nex_state.active_panel {
            PanelId::GROUNDED => self.grounded_panel.paint(area),
            PanelId::HYPOTHESIS => self.hypothesis_panel.paint(area),
            PanelId::EXPERIENCE => self.experience_panel.paint(area),
            PanelId::SIGNAL => self.signal_panel.paint(area),
            PanelId::BRAIN => self.brain_panel.paint(area),
            PanelId::GUARDIAN => self.guardian_panel.paint(area),
            PanelId::MCP => self.mcp_panel.paint(area),
            PanelId::CLOUD => self.cloud_panel.paint(area),
            _ => Vec::new(),
        }
    }

    // ── Phase 2 data fetching ───────────────────────────────────

    /// Fetch brain session data and sync the panel.
    fn fetch_brain_data(&mut self) {
        let result = self.bridge.brain_sessions();
        if let BridgeResult::BrainData {
            sessions,
            artifact_count,
        } = result
        {
            let displays: Vec<SessionDisplay> = sessions.into_iter().map(convert_session).collect();
            self.brain_panel.sync(displays, artifact_count);
        }
    }

    /// Fetch guardian status and sync the panel.
    fn fetch_guardian_data(&mut self) {
        let result = self.bridge.guardian_status();
        if let BridgeResult::GuardianData {
            sensors,
            actuators,
            loop_state,
            risk_level,
        } = result
        {
            let sensor_displays: Vec<SensorDisplay> =
                sensors.into_iter().map(convert_sensor).collect();
            let actuator_displays: Vec<ActuatorDisplay> =
                actuators.into_iter().map(convert_actuator).collect();
            self.guardian_panel
                .sync(sensor_displays, actuator_displays, loop_state, risk_level);
        }
    }

    /// Fetch cloud status data and sync the panel.
    fn fetch_cloud_data(&mut self) {
        let cloud_url = self.cloud_panel.cloud_url().to_string();
        let result = self.bridge.cloud_status(&cloud_url);
        if let BridgeResult::CloudData {
            platform_name,
            services,
            overall_health,
            ..
        } = result
        {
            let displays: Vec<CloudServiceDisplay> =
                services.into_iter().map(convert_cloud_service).collect();
            self.cloud_panel
                .sync(platform_name, displays, overall_health);
        }
    }

    /// Fetch MCP/skill data and sync the panel.
    fn fetch_mcp_data(&mut self) {
        let result = self.bridge.mcp_skills();
        if let BridgeResult::McpData {
            skills,
            total_tools,
        } = result
        {
            let displays: Vec<SkillDisplay> = skills.into_iter().map(convert_skill).collect();
            self.mcp_panel.sync(displays, total_tools);
        }
    }
}

/// Convert a bridge SessionSummary to panel SessionDisplay.
fn convert_session(s: crate::bridge::SessionSummary) -> SessionDisplay {
    SessionDisplay {
        id: s.id,
        created: s.created,
        artifacts: s.artifact_count,
    }
}

/// Convert a bridge SensorStatus to panel SensorDisplay.
fn convert_sensor(s: crate::bridge::SensorStatus) -> SensorDisplay {
    SensorDisplay {
        name: s.name,
        active: s.active,
        alerts: s.alert_count,
    }
}

/// Convert a bridge ActuatorStatus to panel ActuatorDisplay.
fn convert_actuator(a: crate::bridge::ActuatorStatus) -> ActuatorDisplay {
    ActuatorDisplay {
        name: a.name,
        enabled: a.enabled,
    }
}

/// Convert a bridge CloudServiceInfo to panel CloudServiceDisplay.
fn convert_cloud_service(s: crate::state::CloudServiceInfo) -> CloudServiceDisplay {
    CloudServiceDisplay {
        name: s.name,
        state: s.state,
        port: s.port,
        pid: s.pid,
        restarts: s.restarts,
        healthy: s.healthy,
    }
}

/// Convert a bridge SkillSummary to panel SkillDisplay.
fn convert_skill(s: crate::bridge::SkillSummary) -> SkillDisplay {
    SkillDisplay {
        name: s.name,
        category: s.category,
        tools: s.tool_count,
    }
}

/// Run the browser window.
///
/// # Errors
/// Returns error if event loop fails.
pub fn run(url: &str) -> nexcore_error::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::new(url.to_string());
    event_loop.run_app(&mut app)?;
    Ok(())
}
