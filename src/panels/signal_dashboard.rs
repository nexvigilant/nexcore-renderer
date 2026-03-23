//! Signal Dashboard panel — PV signal detection interface.
//!
//! ## Tier Classification
//!
//! - `SignalDashboard`: T3 (domain panel)

use super::Panel;
use crate::chrome::Theme;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Tier: T3 — Signal detection dashboard panel.
pub struct SignalDashboard {
    /// Last signal result (if any).
    last_result: Option<SignalResult>,
}

/// Tier: T2-C — A signal detection result for display.
#[derive(Debug, Clone)]
struct SignalResult {
    drug: String,
    event: String,
    prr: f64,
    ror: f64,
    ic: f64,
    detected: bool,
}

impl SignalDashboard {
    /// Create a new dashboard.
    #[must_use]
    pub fn new() -> Self {
        Self { last_result: None }
    }

    /// Set the last signal result.
    pub fn set_result(
        &mut self,
        drug: String,
        event: String,
        prr: f64,
        ror: f64,
        ic: f64,
        detected: bool,
    ) {
        self.last_result = Some(SignalResult {
            drug,
            event,
            prr,
            ror,
            ic,
            detected,
        });
    }
}

impl Default for SignalDashboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint the result section.
fn paint_result(result: &SignalResult, area: &Rect) -> Vec<DisplayCommand> {
    let status_color = if result.detected {
        Theme::CONFIDENCE_DOWN // Red for detected signal
    } else {
        Theme::CONFIDENCE_UP // Green for no signal
    };
    let status_text = if result.detected {
        "SIGNAL DETECTED"
    } else {
        "No signal"
    };

    vec![
        DisplayCommand::DrawText {
            text: format!("{} / {}", result.drug, result.event),
            x: area.x + 16.0,
            y: area.y + 50.0,
            size: Theme::FONT_SIZE,
            color: Theme::SIDEBAR_TEXT,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: status_text.to_string(),
            x: area.x + 16.0,
            y: area.y + 72.0,
            size: 14.0,
            color: status_color,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!(
                "PRR: {:.2}  ROR: {:.2}  IC: {:.2}",
                result.prr, result.ror, result.ic
            ),
            x: area.x + 16.0,
            y: area.y + 94.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

impl Panel for SignalDashboard {
    fn id(&self) -> PanelId {
        PanelId::SIGNAL
    }
    fn name(&self) -> &str {
        "Signal Dashboard"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = vec![DisplayCommand::DrawText {
            text: "Signal Detection".to_string(),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        }];
        if let Some(result) = &self.last_result {
            cmds.extend(paint_result(result, &area));
        } else {
            cmds.push(DisplayCommand::DrawText {
                text: "No signals checked yet".to_string(),
                x: area.x + 16.0,
                y: area.y + 50.0,
                size: Theme::FONT_SIZE_SMALL,
                color: Theme::STATUS_TEXT,
                node_id: None,
            });
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
    fn test_dashboard_creation() {
        let d = SignalDashboard::new();
        assert!(d.last_result.is_none());
    }

    #[test]
    fn test_dashboard_paint_empty() {
        let d = SignalDashboard::new();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = d.paint(area);
        assert_eq!(cmds.len(), 2); // header + "no signals"
    }
}
