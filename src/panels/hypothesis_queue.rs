//! Hypothesis Queue panel — lists pending hypotheses.
//!
//! ## Tier Classification
//!
//! - `HypothesisQueuePanel`: T3 (domain panel)

use super::Panel;
use crate::chrome::Theme;
use crate::grounded::{Hypothesis, HypothesisId, HypothesisStatus};
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Tier: T3 — Hypothesis queue panel.
pub struct HypothesisQueuePanel {
    /// Snapshot of queued hypotheses for rendering.
    hypotheses: Vec<HypothesisSummary>,
}

/// Tier: T2-C — Display summary of a hypothesis.
#[derive(Debug, Clone)]
struct HypothesisSummary {
    id: HypothesisId,
    claim: String,
    status: String,
    confidence: f64,
}

impl HypothesisQueuePanel {
    /// Create a new panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hypotheses: Vec::new(),
        }
    }

    /// Update from the hypothesis queue.
    pub fn sync(&mut self, queue: &[Hypothesis]) {
        self.hypotheses = queue
            .iter()
            .map(|h| HypothesisSummary {
                id: h.id,
                claim: h.claim.clone(),
                status: format!("{}", h.status),
                confidence: h.confidence.confidence,
            })
            .collect();
    }
}

impl Default for HypothesisQueuePanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint a single hypothesis entry.
fn paint_entry(entry: &HypothesisSummary, index: usize, area: &Rect) -> Vec<DisplayCommand> {
    let y = area.y + 30.0 + (index as f32) * 50.0;
    vec![
        DisplayCommand::DrawText {
            text: format!("{}: {}", entry.id, entry.claim),
            x: area.x + 16.0,
            y,
            size: Theme::FONT_SIZE,
            color: Theme::SIDEBAR_TEXT,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("[{}] {:.0}%", entry.status, entry.confidence * 100.0),
            x: area.x + 16.0,
            y: y + 18.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

impl Panel for HypothesisQueuePanel {
    fn id(&self) -> PanelId {
        PanelId::HYPOTHESIS
    }
    fn name(&self) -> &str {
        "Hypothesis Queue"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = vec![DisplayCommand::DrawText {
            text: format!("Hypotheses ({})", self.hypotheses.len()),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        }];
        for (i, h) in self.hypotheses.iter().enumerate() {
            cmds.extend(paint_entry(h, i, &area));
        }
        cmds
    }

    fn handle_click(&mut self, _x: f32, y: f32, area: Rect) -> Option<Message> {
        let relative_y = y - area.y - 30.0;
        if relative_y < 0.0 {
            return None;
        }
        let index = (relative_y / 50.0) as usize;
        self.hypotheses
            .get(index)
            .map(|h| Message::ApproveHypothesis(h.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = HypothesisQueuePanel::new();
        assert!(panel.hypotheses.is_empty());
    }

    #[test]
    fn test_panel_paint_empty() {
        let panel = HypothesisQueuePanel::new();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        assert_eq!(cmds.len(), 1); // Just the header
    }
}
