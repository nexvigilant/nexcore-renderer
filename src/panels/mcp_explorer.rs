//! MCP Explorer panel — lists available skills and tool counts.
//!
//! ## Tier Classification
//!
//! - `McpExplorerPanel`: T3 (domain panel)
//! - `SkillDisplay`: T2-C (display projection)

use super::Panel;
use crate::chrome::Theme;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};

/// Max skills displayed in the list.
const MAX_DISPLAY_SKILLS: usize = 15;

/// Tier: T2-C — Skill data projected for display.
#[derive(Debug, Clone)]
pub struct SkillDisplay {
    /// Skill name.
    pub name: String,
    /// Skill category.
    pub category: String,
    /// Number of tools provided by this skill.
    pub tools: usize,
}

/// Tier: T3 — MCP/Skill explorer panel.
pub struct McpExplorerPanel {
    /// Skills available.
    skills: Vec<SkillDisplay>,
    /// Total tool count across all skills.
    total_tools: usize,
}

impl McpExplorerPanel {
    /// Create a new empty MCP explorer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            total_tools: 0,
        }
    }

    /// Sync display data from bridge results.
    pub fn sync(&mut self, skills: Vec<SkillDisplay>, total_tools: usize) {
        self.skills = skills;
        self.total_tools = total_tools;
    }
}

impl Default for McpExplorerPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Paint the panel header with counts.
fn paint_header(area: &Rect, skill_count: usize, total_tools: usize) -> Vec<DisplayCommand> {
    vec![
        DisplayCommand::DrawText {
            text: "MCP — Skills & Tools".to_string(),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("{skill_count} skills, {total_tools} tools"),
            x: area.x + 16.0,
            y: area.y + 42.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

/// Paint a single skill entry.
fn paint_skill(area: &Rect, skill: &SkillDisplay, index: usize) -> Vec<DisplayCommand> {
    let y_offset = 65.0 + (index as f32 * 30.0);
    vec![
        DisplayCommand::DrawText {
            text: skill.name.clone(),
            x: area.x + 16.0,
            y: area.y + y_offset,
            size: Theme::FONT_SIZE,
            color: Theme::SIDEBAR_TEXT,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("{} — {} tools", skill.category, skill.tools),
            x: area.x + 16.0,
            y: area.y + y_offset + 14.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

impl Panel for McpExplorerPanel {
    fn id(&self) -> PanelId {
        PanelId::MCP
    }

    fn name(&self) -> &str {
        "MCP Explorer"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = paint_header(&area, self.skills.len(), self.total_tools);
        if self.skills.is_empty() {
            cmds.push(DisplayCommand::DrawText {
                text: "No skills loaded".to_string(),
                x: area.x + 16.0,
                y: area.y + 65.0,
                size: Theme::FONT_SIZE_SMALL,
                color: Theme::STATUS_TEXT,
                node_id: None,
            });
            return cmds;
        }
        let limit = self.skills.len().min(MAX_DISPLAY_SKILLS);
        for (i, skill) in self.skills[..limit].iter().enumerate() {
            cmds.extend(paint_skill(&area, skill, i));
        }
        cmds
    }

    fn handle_click(&mut self, _x: f32, _y: f32, _area: Rect) -> Option<Message> {
        Some(Message::Noop) // Phase 3: inspect skill on click
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_creation() {
        let panel = McpExplorerPanel::new();
        assert!(panel.skills.is_empty());
        assert_eq!(panel.total_tools, 0);
    }

    #[test]
    fn test_mcp_sync() {
        let mut panel = McpExplorerPanel::new();
        panel.sync(
            vec![SkillDisplay {
                name: "forge".to_string(),
                category: "dev".to_string(),
                tools: 5,
            }],
            112,
        );
        assert_eq!(panel.skills.len(), 1);
        assert_eq!(panel.total_tools, 112);
    }

    #[test]
    fn test_mcp_paint_empty() {
        let panel = McpExplorerPanel::new();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        assert_eq!(cmds.len(), 3); // header(2) + "no skills"(1)
    }

    #[test]
    fn test_mcp_paint_with_skills() {
        let mut panel = McpExplorerPanel::new();
        panel.sync(
            vec![
                SkillDisplay {
                    name: "forge".to_string(),
                    category: "dev".to_string(),
                    tools: 5,
                },
                SkillDisplay {
                    name: "brain".to_string(),
                    category: "memory".to_string(),
                    tools: 8,
                },
            ],
            13,
        );
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        // header(2) + 2 skills * 2 cmds each = 6
        assert_eq!(cmds.len(), 6);
    }
}
