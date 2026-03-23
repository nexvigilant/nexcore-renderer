//! Brain Viewer panel — displays session and artifact data from nexcore-brain.
//!
//! ## Tier Classification
//!
//! - `BrainViewerPanel`: T3 (domain panel)
//! - `SessionDisplay`: T2-C (display projection)

use super::Panel;
use crate::chrome::Theme;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Max sessions displayed in the list.
const MAX_DISPLAY_SESSIONS: usize = 10;

/// Tier: T2-C — Session data projected for display.
#[derive(Debug, Clone)]
pub struct SessionDisplay {
    /// Session identifier (may be truncated for display).
    pub id: String,
    /// Creation timestamp.
    pub created: String,
    /// Number of artifacts.
    pub artifacts: usize,
}

/// Tier: T3 — Brain session and artifact viewer panel.
pub struct BrainViewerPanel {
    /// Sessions available for display.
    sessions: Vec<SessionDisplay>,
    /// Total artifact count across all sessions.
    artifact_count: usize,
}

impl BrainViewerPanel {
    /// Create a new empty brain viewer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            artifact_count: 0,
        }
    }

    /// Sync display data from bridge results.
    pub fn sync(&mut self, sessions: Vec<SessionDisplay>, artifact_count: usize) {
        self.sessions = sessions;
        self.artifact_count = artifact_count;
    }
}

impl Default for BrainViewerPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint the panel header with title and counts.
fn paint_header(area: &Rect, session_count: usize, artifact_count: usize) -> Vec<DisplayCommand> {
    vec![
        DisplayCommand::DrawText {
            text: "Brain — Sessions".to_string(),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("{session_count} sessions, {artifact_count} artifacts"),
            x: area.x + 16.0,
            y: area.y + 42.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

/// Paint a single session entry.
fn paint_session(area: &Rect, session: &SessionDisplay, index: usize) -> Vec<DisplayCommand> {
    let y_offset = 65.0 + (index as f32 * 36.0);
    let display_id = truncate_id(&session.id, 12);
    let ts = format_timestamp(&session.created);
    vec![
        DisplayCommand::DrawText {
            text: display_id,
            x: area.x + 16.0,
            y: area.y + y_offset,
            size: Theme::FONT_SIZE,
            color: Theme::SIDEBAR_TEXT,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("{ts}  ({} artifacts)", session.artifacts),
            x: area.x + 16.0,
            y: area.y + y_offset + 16.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

/// Truncate an ID string for display.
fn truncate_id(id: &str, max_len: usize) -> String {
    if id.len() <= max_len {
        return id.to_string();
    }
    format!("{}…", &id[..max_len])
}

/// Format a timestamp for compact display (take first 10 chars = date).
fn format_timestamp(ts: &str) -> String {
    if ts.len() >= 10 {
        return ts[..10].to_string();
    }
    ts.to_string()
}

impl Panel for BrainViewerPanel {
    fn id(&self) -> PanelId {
        PanelId::BRAIN
    }

    fn name(&self) -> &str {
        "Brain Viewer"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = paint_header(&area, self.sessions.len(), self.artifact_count);
        if self.sessions.is_empty() {
            cmds.push(DisplayCommand::DrawText {
                text: "No sessions loaded".to_string(),
                x: area.x + 16.0,
                y: area.y + 65.0,
                size: Theme::FONT_SIZE_SMALL,
                color: Theme::STATUS_TEXT,
                node_id: None,
            });
            return cmds;
        }
        let limit = self.sessions.len().min(MAX_DISPLAY_SESSIONS);
        for (i, session) in self.sessions[..limit].iter().enumerate() {
            cmds.extend(paint_session(&area, session, i));
        }
        cmds
    }

    fn handle_click(&mut self, _x: f32, _y: f32, _area: Rect) -> Option<Message> {
        Some(Message::Noop) // Phase 3: load session on click
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brain_viewer_creation() {
        let panel = BrainViewerPanel::new();
        assert!(panel.sessions.is_empty());
        assert_eq!(panel.artifact_count, 0);
    }

    #[test]
    fn test_brain_viewer_sync() {
        let mut panel = BrainViewerPanel::new();
        let sessions = vec![SessionDisplay {
            id: "sess-001".to_string(),
            created: "2026-01-15T10:00:00Z".to_string(),
            artifacts: 5,
        }];
        panel.sync(sessions, 42);
        assert_eq!(panel.sessions.len(), 1);
        assert_eq!(panel.artifact_count, 42);
    }

    #[test]
    fn test_brain_viewer_paint_empty() {
        let panel = BrainViewerPanel::new();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        assert_eq!(cmds.len(), 3); // header(2) + "no sessions"(1)
    }

    #[test]
    fn test_brain_viewer_paint_with_sessions() {
        let mut panel = BrainViewerPanel::new();
        panel.sync(
            vec![
                SessionDisplay {
                    id: "s1".to_string(),
                    created: "2026-01-01".to_string(),
                    artifacts: 3,
                },
                SessionDisplay {
                    id: "s2".to_string(),
                    created: "2026-01-02".to_string(),
                    artifacts: 7,
                },
            ],
            10,
        );
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        // header(2) + 2 sessions * 2 cmds each = 6
        assert_eq!(cmds.len(), 6);
    }

    #[test]
    fn test_truncate_id() {
        assert_eq!(truncate_id("short", 12), "short");
        assert_eq!(truncate_id("very-long-session-id", 12), "very-long-se…");
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp("2026-01-15T10:00:00Z"), "2026-01-15");
        assert_eq!(format_timestamp("short"), "short");
    }
}
