//! Panel system — sidebar panels for GROUNDED loop visualization.
//!
//! Each panel implements the `Panel` trait and renders into the
//! sidebar content area. Panels are selected via the sidebar menu.
//!
//! ## Tier Classification
//!
//! - `Panel`: T2-C (trait)
//! - Individual panels: T3 (domain-specific)

pub mod brain_viewer;
pub mod cloud_dashboard;
pub mod experience_store;
pub mod grounded_monitor;
pub mod guardian_monitor;
pub mod hypothesis_queue;
pub mod mcp_explorer;
pub mod signal_dashboard;

use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Tier: T2-C — Trait for sidebar panels.
pub trait Panel {
    /// Panel identifier.
    fn id(&self) -> PanelId;

    /// Panel display name.
    fn name(&self) -> &str;

    /// Render the panel content into display commands.
    fn paint(&self, area: Rect) -> Vec<DisplayCommand>;

    /// Handle a click within the panel area.
    fn handle_click(&mut self, x: f32, y: f32, area: Rect) -> Option<Message>;
}
