//! Cloud Dashboard panel — NexCloud service status visualization.
//!
//! ## Tier Classification
//!
//! - `CloudDashboardPanel`: T3 (domain panel)
//! - `CloudServiceDisplay`: T2-C (display projection)

use super::Panel;
use crate::chrome::Theme;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};
use crate::style::Color;

/// Tier: T2-C — Service data projected for display.
#[derive(Debug, Clone)]
pub struct CloudServiceDisplay {
    /// Service name.
    pub name: String,
    /// Current lifecycle state (e.g. "healthy", "unhealthy").
    pub state: String,
    /// Listening port.
    pub port: u16,
    /// Process ID.
    pub pid: Option<u32>,
    /// Number of restarts.
    pub restarts: u32,
    /// Whether the service is healthy.
    pub healthy: bool,
}

/// Tier: T3 — Cloud status dashboard panel.
pub struct CloudDashboardPanel {
    /// Platform name.
    platform_name: String,
    /// Overall health string ("healthy", "degraded", "critical").
    overall_health: String,
    /// Per-service display data.
    services: Vec<CloudServiceDisplay>,
    /// NexCloud status endpoint URL.
    cloud_url: String,
}

impl CloudDashboardPanel {
    /// Create a new cloud dashboard panel.
    #[must_use]
    pub fn new(cloud_url: impl Into<String>) -> Self {
        Self {
            platform_name: "unknown".to_string(),
            overall_health: "unknown".to_string(),
            services: Vec::new(),
            cloud_url: cloud_url.into(),
        }
    }

    /// Sync panel data from bridge results.
    pub fn sync(
        &mut self,
        platform_name: String,
        services: Vec<CloudServiceDisplay>,
        overall_health: String,
    ) {
        self.platform_name = platform_name;
        self.services = services;
        self.overall_health = overall_health;
    }

    /// Get the cloud status URL.
    #[must_use]
    pub fn cloud_url(&self) -> &str {
        &self.cloud_url
    }
}

impl Default for CloudDashboardPanel {
    fn default() -> Self {
        let url =
            std::env::var("NEXCLOUD_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        Self::new(url)
    }
}

/// Color for a health state string.
fn health_color(state: &str) -> Color {
    match state {
        "healthy" => Theme::CONFIDENCE_UP, // green
        "critical" | "FAILED" | "unhealthy" => Theme::CONFIDENCE_DOWN, // red
        "degraded" | "restarting" | "starting" => Theme::STATUS_TEXT, // gray/yellow
        _ => Theme::SIDEBAR_TEXT,
    }
}

/// Paint the panel header with platform name and overall health.
fn paint_header(area: &Rect, platform_name: &str, overall_health: &str) -> Vec<DisplayCommand> {
    let health_col = health_color(overall_health);
    vec![
        DisplayCommand::DrawText {
            text: format!("\u{2601} Cloud \u{2014} {platform_name}"),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("Health: {overall_health}"),
            x: area.x + 16.0,
            y: area.y + 42.0,
            size: Theme::FONT_SIZE_SMALL,
            color: health_col,
            node_id: None,
        },
    ]
}

/// Paint column headers for the service table.
fn paint_table_header(area: &Rect, y_offset: f32) -> DisplayCommand {
    DisplayCommand::DrawText {
        text: "Service        State     Port   PID     Restarts".to_string(),
        x: area.x + 16.0,
        y: area.y + y_offset,
        size: Theme::FONT_SIZE_SMALL,
        color: Theme::SIDEBAR_ACTIVE,
        node_id: None,
    }
}

/// Paint a single service row.
fn paint_service_row(area: &Rect, svc: &CloudServiceDisplay, y_offset: f32) -> DisplayCommand {
    let indicator = if svc.healthy { "\u{25CF}" } else { "\u{25CB}" };
    let pid_str = svc
        .pid
        .map(|p| p.to_string())
        .unwrap_or_else(|| "-".to_string());
    let text = format!(
        "{indicator} {:<13} {:<9} {:<6} {:<7} {}",
        svc.name, svc.state, svc.port, pid_str, svc.restarts
    );
    let color = health_color(&svc.state);
    DisplayCommand::DrawText {
        text,
        x: area.x + 16.0,
        y: area.y + y_offset,
        size: Theme::FONT_SIZE_SMALL,
        color,
        node_id: None,
    }
}

/// Paint a service count summary.
fn paint_summary(area: &Rect, total: usize, healthy: usize, y_offset: f32) -> DisplayCommand {
    DisplayCommand::DrawText {
        text: format!("{healthy}/{total} services healthy"),
        x: area.x + 16.0,
        y: area.y + y_offset,
        size: Theme::FONT_SIZE,
        color: Theme::SIDEBAR_TEXT,
        node_id: None,
    }
}

impl Panel for CloudDashboardPanel {
    fn id(&self) -> PanelId {
        PanelId::CLOUD
    }

    fn name(&self) -> &str {
        "Cloud Status"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = paint_header(&area, &self.platform_name, &self.overall_health);

        let healthy_count = self.services.iter().filter(|s| s.healthy).count();
        cmds.push(paint_summary(
            &area,
            self.services.len(),
            healthy_count,
            62.0,
        ));

        if !self.services.is_empty() {
            cmds.push(paint_table_header(&area, 90.0));

            let mut y = 108.0;
            for svc in &self.services {
                cmds.push(paint_service_row(&area, svc, y));
                y += 18.0;
            }
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
    fn test_panel_id() {
        let panel = CloudDashboardPanel::new("http://localhost:8080");
        assert_eq!(panel.id(), PanelId::CLOUD);
    }

    #[test]
    fn test_panel_name() {
        let panel = CloudDashboardPanel::new("http://localhost:8080");
        assert_eq!(panel.name(), "Cloud Status");
    }

    #[test]
    fn test_sync() {
        let mut panel = CloudDashboardPanel::new("http://localhost:8080");
        panel.sync(
            "test-platform".to_string(),
            vec![CloudServiceDisplay {
                name: "web".to_string(),
                state: "healthy".to_string(),
                port: 8080,
                pid: Some(1234),
                restarts: 0,
                healthy: true,
            }],
            "healthy".to_string(),
        );
        assert_eq!(panel.platform_name, "test-platform");
        assert_eq!(panel.services.len(), 1);
        assert_eq!(panel.overall_health, "healthy");
    }

    #[test]
    fn test_paint_empty() {
        let panel = CloudDashboardPanel::new("http://localhost:8080");
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        // header(2) + summary(1) = 3
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn test_paint_with_services() {
        let mut panel = CloudDashboardPanel::new("http://localhost:8080");
        panel.sync(
            "prod".to_string(),
            vec![
                CloudServiceDisplay {
                    name: "api".to_string(),
                    state: "healthy".to_string(),
                    port: 3030,
                    pid: Some(100),
                    restarts: 0,
                    healthy: true,
                },
                CloudServiceDisplay {
                    name: "worker".to_string(),
                    state: "unhealthy".to_string(),
                    port: 3031,
                    pid: Some(101),
                    restarts: 2,
                    healthy: false,
                },
            ],
            "degraded".to_string(),
        );
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        // header(2) + summary(1) + table_header(1) + rows(2) = 6
        assert_eq!(cmds.len(), 6);
    }

    #[test]
    fn test_health_color_healthy() {
        let color = health_color("healthy");
        assert_eq!(color.r, Theme::CONFIDENCE_UP.r);
    }

    #[test]
    fn test_health_color_critical() {
        let color = health_color("critical");
        assert_eq!(color.r, Theme::CONFIDENCE_DOWN.r);
    }

    #[test]
    fn test_cloud_url() {
        let panel = CloudDashboardPanel::new("http://custom:9090");
        assert_eq!(panel.cloud_url(), "http://custom:9090");
    }

    #[test]
    fn test_default_cloud_url_fallback() {
        // Without NEXCLOUD_URL env var set, should use localhost:8080
        let panel = CloudDashboardPanel::default();
        assert_eq!(panel.cloud_url(), "http://localhost:8080");
    }

    #[test]
    fn test_handle_click_returns_none() {
        let mut panel = CloudDashboardPanel::new("http://localhost:8080");
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        assert!(panel.handle_click(10.0, 10.0, area).is_none());
    }
}
