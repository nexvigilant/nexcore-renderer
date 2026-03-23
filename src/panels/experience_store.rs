//! Experience Store panel — searchable experience history.
//!
//! ## Tier Classification
//!
//! - `ExperienceStorePanel`: T3 (domain panel)

use super::Panel;
use crate::chrome::Theme;
use crate::grounded::experience::ExperienceStore;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Tier: T3 — Experience store browser panel.
pub struct ExperienceStorePanel {
    /// Total experience count.
    count: usize,
    /// Confirmation rate.
    confirmation_rate: f64,
    /// Recent experience summaries (claim + outcome).
    recent: Vec<(String, bool)>,
}

impl ExperienceStorePanel {
    /// Create a new panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            count: 0,
            confirmation_rate: 0.0,
            recent: Vec::new(),
        }
    }

    /// Update from the experience store.
    pub fn sync(&mut self, store: &ExperienceStore) {
        self.count = store.count();
        self.confirmation_rate = store.confirmation_rate();
        self.recent = store
            .recent(10)
            .iter()
            .map(|e| (e.hypothesis.clone(), e.outcome.supported))
            .collect();
    }
}

impl Default for ExperienceStorePanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint the stats header.
fn paint_stats(area: &Rect, count: usize, rate: f64) -> Vec<DisplayCommand> {
    vec![
        DisplayCommand::DrawText {
            text: format!("Experiences ({count})"),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("Confirmation rate: {:.0}%", rate * 100.0),
            x: area.x + 16.0,
            y: area.y + 42.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

/// Paint a single recent experience.
fn paint_recent_entry(claim: &str, supported: bool, index: usize, area: &Rect) -> DisplayCommand {
    let icon = if supported { "✓" } else { "✗" };
    let color = if supported {
        Theme::CONFIDENCE_UP
    } else {
        Theme::CONFIDENCE_DOWN
    };
    let y = area.y + 65.0 + (index as f32) * 22.0;
    let label = if claim.len() > 30 {
        format!("{icon} {}...", &claim[..30])
    } else {
        format!("{icon} {claim}")
    };
    DisplayCommand::DrawText {
        text: label,
        x: area.x + 16.0,
        y,
        size: Theme::FONT_SIZE_SMALL,
        color,
        node_id: None,
    }
}

impl Panel for ExperienceStorePanel {
    fn id(&self) -> PanelId {
        PanelId::EXPERIENCE
    }
    fn name(&self) -> &str {
        "Experience Store"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = paint_stats(&area, self.count, self.confirmation_rate);
        for (i, (claim, supported)) in self.recent.iter().enumerate() {
            cmds.push(paint_recent_entry(claim, *supported, i, &area));
        }
        cmds
    }

    fn handle_click(&mut self, _x: f32, _y: f32, _area: Rect) -> Option<Message> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = ExperienceStorePanel::new();
        assert_eq!(panel.count, 0);
    }
}
