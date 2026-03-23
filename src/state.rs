//! NexBrowser state management.
//!
//! Central state for the browser, using a message-driven architecture.
//! All user actions and system events are represented as `Message` values,
//! and side effects are represented as `Effect` values.
//!
//! ## Tier Classification
//!
//! - `TabId`, `PanelId`, `WidgetId`: T2-P (newtype identifiers)
//! - `Message`: T2-C (sum type for all state transitions)
//! - `NexBrowserState`: T3 (domain-specific application state)

use crate::bridge::{ActuatorStatus, SensorStatus, SessionSummary, SkillSummary};
use crate::grounded::{GroundedLoop, Hypothesis, HypothesisId, Learning, Outcome};

/// Tier: T2-P — Unique identifier for a browser tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u32);

/// Tier: T2-P — Unique identifier for a sidebar panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub u32);

/// Tier: T2-P — Unique identifier for a chrome widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u32);

/// Well-known panel IDs.
impl PanelId {
    /// GROUNDED Loop Monitor.
    pub const GROUNDED: Self = Self(0);
    /// Hypothesis Queue.
    pub const HYPOTHESIS: Self = Self(1);
    /// Experience Store.
    pub const EXPERIENCE: Self = Self(2);
    /// Signal Dashboard.
    pub const SIGNAL: Self = Self(3);
    /// Brain Viewer (Phase 2).
    pub const BRAIN: Self = Self(4);
    /// Guardian Monitor (Phase 2).
    pub const GUARDIAN: Self = Self(5);
    /// MCP Tools (Phase 2).
    pub const MCP: Self = Self(6);
    /// Cloud Status (Phase 2).
    pub const CLOUD: Self = Self(7);
}

/// Tier: T2-C — All possible state transitions.
///
/// Every user action, system event, and GROUNDED loop event
/// is represented as a `Message`.
#[derive(Debug, Clone)]
pub enum Message {
    // ── Browser navigation ─────────────────────────────────────
    /// Navigate the active tab to a URL.
    Navigate(String),
    /// Open a new tab, optionally with a URL.
    NewTab(Option<String>),
    /// Close a tab.
    CloseTab(TabId),
    /// Switch to a tab.
    SelectTab(TabId),
    /// Go back in history.
    GoBack,
    /// Go forward in history.
    GoForward,
    /// Reload current page.
    Reload,

    // ── Chrome UI ──────────────────────────────────────────────
    /// Toggle sidebar visibility.
    ToggleSidebar,
    /// Select a panel in the sidebar.
    SelectPanel(PanelId),
    /// Scroll content area.
    Scroll { dy: f32 },
    /// Zoom content area.
    Zoom(f32),
    /// Click in content area.
    Click { x: f32, y: f32 },
    /// Focus address bar.
    FocusAddressBar,
    /// Window resized.
    Resize(u32, u32),

    // ── GROUNDED loop ──────────────────────────────────────────
    /// Propose a new hypothesis.
    ProposeHypothesis(Hypothesis),
    /// Approve a hypothesis for testing.
    ApproveHypothesis(HypothesisId),
    /// Start running an experiment.
    RunExperiment(HypothesisId),
    /// Experiment completed with outcome.
    ExperimentComplete(HypothesisId, Outcome),
    /// Integrate a learning into context.
    IntegrateLearning(Learning),

    // ── Bridge ─────────────────────────────────────────────────
    /// Response from NexCore API bridge.
    BridgeResponse(BridgeResult),

    // ── System ─────────────────────────────────────────────────
    /// Quit the application.
    Quit,
    /// No-op (absorbs unhandled events).
    Noop,
}

/// Tier: T2-C — Result from the NexCore API bridge.
#[derive(Debug, Clone)]
pub enum BridgeResult {
    /// Signal detection result.
    SignalResult {
        drug: String,
        event: String,
        prr: f64,
        ror: f64,
        ic: f64,
        signal_detected: bool,
    },
    /// Generic API response.
    ApiResponse {
        endpoint: String,
        status: u16,
        body: String,
    },
    /// Brain session data (Phase 2).
    BrainData {
        sessions: Vec<SessionSummary>,
        artifact_count: usize,
    },
    /// Guardian status data (Phase 2).
    GuardianData {
        sensors: Vec<SensorStatus>,
        actuators: Vec<ActuatorStatus>,
        loop_state: String,
        risk_level: f64,
    },
    /// MCP/Skill data (Phase 2).
    McpData {
        skills: Vec<SkillSummary>,
        total_tools: usize,
    },
    /// Cloud status data (Phase 2).
    CloudData {
        platform_name: String,
        services: Vec<CloudServiceInfo>,
        overall_health: String,
        service_count: usize,
    },
    /// Bridge error.
    Error(String),
}

/// Tier: T2-C — Cloud service info for bridge results.
#[derive(Debug, Clone)]
pub struct CloudServiceInfo {
    /// Service name.
    pub name: String,
    /// Current state string.
    pub state: String,
    /// Listening port.
    pub port: u16,
    /// Process ID.
    pub pid: Option<u32>,
    /// Restart count.
    pub restarts: u32,
    /// Whether the service is healthy.
    pub healthy: bool,
}

/// Tier: T2-C — Side effects produced by state updates.
///
/// Effects are actions that need to happen outside the pure state update,
/// such as network requests, GPU operations, or process spawning.
#[derive(Debug)]
pub enum Effect {
    /// Fetch a URL and navigate.
    FetchAndNavigate(String),
    /// Call the NexCore API bridge.
    BridgeCall { endpoint: String, payload: String },
    /// Request a frame redraw.
    RequestRedraw,
    /// Exit the event loop.
    ExitApp,
    /// No effect.
    None,
}

/// Tier: T3 — Central application state.
///
/// Wraps the rendering browser, GROUNDED loop, and UI state.
pub struct NexBrowserState {
    /// The GROUNDED reasoning loop.
    pub grounded: GroundedLoop,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
    /// Currently selected panel.
    pub active_panel: PanelId,
    /// Current window dimensions.
    pub window_width: u32,
    /// Current window height.
    pub window_height: u32,
    /// Next tab ID counter.
    next_tab_id: u32,
}

impl Default for NexBrowserState {
    fn default() -> Self {
        Self::new()
    }
}

impl NexBrowserState {
    /// Create a new browser state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            grounded: GroundedLoop::new(),
            sidebar_visible: true,
            active_panel: PanelId::GROUNDED,
            window_width: 1280,
            window_height: 720,
            next_tab_id: 1,
        }
    }

    /// Process a message and return any effects.
    pub fn update(&mut self, msg: Message) -> Effect {
        match msg {
            Message::Navigate(url) => Effect::FetchAndNavigate(url),
            Message::ToggleSidebar => self.handle_toggle_sidebar(),
            Message::SelectPanel(panel) => self.handle_select_panel(panel),
            Message::Resize(w, h) => self.handle_resize(w, h),
            Message::ProposeHypothesis(h) => self.handle_propose(h),
            Message::ApproveHypothesis(id) => self.handle_approve(id),
            Message::RunExperiment(_id) => self.handle_run_experiment(),
            Message::ExperimentComplete(_id, outcome) => self.handle_complete(outcome),
            Message::BridgeResponse(result) => self.handle_bridge(result),
            Message::IntegrateLearning(learning) => self.handle_learning(learning),
            Message::Quit => Effect::ExitApp,
            // Navigation-class messages (GoBack, GoForward, Reload, Scroll, Zoom,
            // Click, NewTab, CloseTab, SelectTab, FocusAddressBar) are dispatched
            // directly to the Browser in window.rs — they do not pass through the
            // state machine. Noop is intentionally a no-op.
            _ => Effect::None,
        }
    }

    fn handle_toggle_sidebar(&mut self) -> Effect {
        self.sidebar_visible = !self.sidebar_visible;
        Effect::RequestRedraw
    }

    fn handle_select_panel(&mut self, panel: PanelId) -> Effect {
        self.active_panel = panel;
        Effect::RequestRedraw
    }

    fn handle_resize(&mut self, w: u32, h: u32) -> Effect {
        self.window_width = w;
        self.window_height = h;
        Effect::RequestRedraw
    }

    fn handle_propose(&mut self, h: Hypothesis) -> Effect {
        self.grounded.propose(h);
        Effect::RequestRedraw
    }

    fn handle_approve(&mut self, id: HypothesisId) -> Effect {
        self.grounded.approve(id);
        Effect::RequestRedraw
    }

    fn handle_run_experiment(&mut self) -> Effect {
        self.grounded.start_next();
        Effect::RequestRedraw
    }

    fn handle_complete(&mut self, outcome: Outcome) -> Effect {
        self.grounded.complete(outcome);
        Effect::RequestRedraw
    }

    fn handle_learning(&mut self, learning: Learning) -> Effect {
        self.grounded.integrate_learning(learning);
        Effect::RequestRedraw
    }

    fn handle_bridge(&mut self, result: BridgeResult) -> Effect {
        log_bridge_result(&result);
        Effect::RequestRedraw
    }

    /// Generate the next unique tab ID.
    pub fn next_tab_id(&mut self) -> TabId {
        let id = TabId(self.next_tab_id);
        self.next_tab_id += 1;
        id
    }

    /// Width available for the content area.
    #[must_use]
    pub fn content_width(&self) -> f32 {
        let sidebar = if self.sidebar_visible { 280.0 } else { 0.0 };
        (self.window_width as f32 - sidebar).max(100.0)
    }

    /// Height available for the content area.
    #[must_use]
    pub fn content_height(&self) -> f32 {
        // tab_bar(36) + toolbar(40) + content + status_bar(24)
        (self.window_height as f32 - 100.0).max(100.0)
    }
}

/// Log a bridge result without deep nesting.
fn log_bridge_result(result: &BridgeResult) {
    match result {
        BridgeResult::SignalResult {
            drug, event, prr, ..
        } => {
            tracing::info!("Signal: {drug}/{event}, PRR={prr:.2}");
        }
        BridgeResult::ApiResponse {
            endpoint, status, ..
        } => {
            tracing::info!("API: {endpoint} -> {status}");
        }
        BridgeResult::BrainData {
            sessions,
            artifact_count,
        } => {
            tracing::info!(
                "Brain: {} sessions, {artifact_count} artifacts",
                sessions.len()
            );
        }
        BridgeResult::GuardianData {
            loop_state,
            risk_level,
            ..
        } => {
            tracing::info!("Guardian: {loop_state}, risk={risk_level:.2}");
        }
        BridgeResult::McpData {
            skills,
            total_tools,
        } => {
            tracing::info!("MCP: {} skills, {total_tools} tools", skills.len());
        }
        BridgeResult::CloudData {
            platform_name,
            service_count,
            overall_health,
            ..
        } => {
            tracing::info!(
                "Cloud: {platform_name}, {service_count} services, health={overall_health}"
            );
        }
        BridgeResult::Error(e) => {
            tracing::error!("Bridge error: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_sidebar() {
        let mut state = NexBrowserState::new();
        assert!(state.sidebar_visible);
        state.update(Message::ToggleSidebar);
        assert!(!state.sidebar_visible);
        state.update(Message::ToggleSidebar);
        assert!(state.sidebar_visible);
    }

    #[test]
    fn test_select_panel() {
        let mut state = NexBrowserState::new();
        state.update(Message::SelectPanel(PanelId::SIGNAL));
        assert_eq!(state.active_panel, PanelId::SIGNAL);
    }

    #[test]
    fn test_content_dimensions() {
        let state = NexBrowserState::new();
        assert!((state.content_width() - 1000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_content_width_no_sidebar() {
        let mut state = NexBrowserState::new();
        state.sidebar_visible = false;
        assert!((state.content_width() - 1280.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quit_produces_exit_effect() {
        let mut state = NexBrowserState::new();
        let effect = state.update(Message::Quit);
        assert!(matches!(effect, Effect::ExitApp));
    }

    #[test]
    fn test_tab_id_generation() {
        let mut state = NexBrowserState::new();
        let id1 = state.next_tab_id();
        let id2 = state.next_tab_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_bridge_result_phase2_variants() {
        // Just verify the variants compile and can be constructed
        let brain = BridgeResult::BrainData {
            sessions: vec![],
            artifact_count: 0,
        };
        assert!(matches!(brain, BridgeResult::BrainData { .. }));

        let guardian = BridgeResult::GuardianData {
            sensors: vec![],
            actuators: vec![],
            loop_state: "idle".to_string(),
            risk_level: 0.0,
        };
        assert!(matches!(guardian, BridgeResult::GuardianData { .. }));

        let mcp = BridgeResult::McpData {
            skills: vec![],
            total_tools: 0,
        };
        assert!(matches!(mcp, BridgeResult::McpData { .. }));
    }

    #[test]
    fn test_grounded_integration() {
        let mut state = NexBrowserState::new();
        let h = Hypothesis::new("test claim", "falsification");
        let hid = h.id;
        state.update(Message::ProposeHypothesis(h));
        assert_eq!(state.grounded.queue_len(), 1);

        state.update(Message::ApproveHypothesis(hid));
        state.update(Message::RunExperiment(hid));

        let outcome = Outcome::success("confirmed", 0.8, 100);
        state.update(Message::ExperimentComplete(hid, outcome));
        assert_eq!(state.grounded.cycle_count(), 1);
    }
}
