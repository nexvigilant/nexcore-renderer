//! GROUNDED Loop Monitor panel — visualizes the active cycle.
//!
//! ## Tier Classification
//!
//! - `GroundedMonitor`: T3 (domain panel)

use super::Panel;
use crate::chrome::Theme;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Tier: T3 — GROUNDED loop visualizer panel.
pub struct GroundedMonitor {
    /// Current cycle count.
    pub cycle: u64,
    /// Current confidence.
    pub confidence: f64,
    /// Active hypothesis claim (if any).
    pub active_claim: Option<String>,
    /// Queue length.
    pub queue_len: usize,
    /// Learning count.
    pub learning_count: usize,
}

impl GroundedMonitor {
    /// Create a new monitor panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cycle: 0,
            confidence: 0.5,
            active_claim: None,
            queue_len: 0,
            learning_count: 0,
        }
    }

    /// Update from GROUNDED loop state.
    pub fn sync(
        &mut self,
        cycle: u64,
        confidence: f64,
        claim: Option<String>,
        queue: usize,
        learnings: usize,
    ) {
        self.cycle = cycle;
        self.confidence = confidence;
        self.active_claim = claim;
        self.queue_len = queue;
        self.learning_count = learnings;
    }
}

impl Default for GroundedMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint the cycle header section.
fn paint_header(area: &Rect, cycle: u64, confidence: f64) -> Vec<DisplayCommand> {
    vec![
        DisplayCommand::DrawText {
            text: format!("GROUNDED Loop - Cycle {cycle}"),
            x: area.x + 16.0,
            y: area.y + 30.0,
            size: 16.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("Confidence: {:.0}%", confidence * 100.0),
            x: area.x + 16.0,
            y: area.y + 55.0,
            size: Theme::FONT_SIZE,
            color: Theme::SIDEBAR_TEXT,
            node_id: None,
        },
    ]
}

/// Paint the active hypothesis section.
fn paint_active(area: &Rect, claim: &Option<String>) -> DisplayCommand {
    let text = match claim {
        Some(c) => format!("Active: {c}"),
        None => "No active hypothesis".to_string(),
    };
    DisplayCommand::DrawText {
        text,
        x: area.x + 16.0,
        y: area.y + 85.0,
        size: Theme::FONT_SIZE,
        color: Theme::SIDEBAR_TEXT,
        node_id: None,
    }
}

/// Paint the stats section.
fn paint_stats(area: &Rect, queue: usize, learnings: usize) -> Vec<DisplayCommand> {
    vec![
        DisplayCommand::DrawText {
            text: format!("Queue: {queue} hypotheses"),
            x: area.x + 16.0,
            y: area.y + 115.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("Learnings: {learnings}"),
            x: area.x + 16.0,
            y: area.y + 135.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

impl Panel for GroundedMonitor {
    fn id(&self) -> PanelId {
        PanelId::GROUNDED
    }
    fn name(&self) -> &str {
        "GROUNDED Loop"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = Vec::with_capacity(8);
        cmds.extend(paint_header(&area, self.cycle, self.confidence));
        cmds.push(paint_active(&area, &self.active_claim));
        cmds.extend(paint_stats(&area, self.queue_len, self.learning_count));
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
    fn test_monitor_creation() {
        let m = GroundedMonitor::new();
        assert_eq!(m.cycle, 0);
        assert!((m.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_monitor_paint() {
        let m = GroundedMonitor::new();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = m.paint(area);
        assert!(!cmds.is_empty());
    }
}
