//! Guardian Monitor panel — live homeostasis loop visualization.
//!
//! ## Tier Classification
//!
//! - `GuardianMonitorPanel`: T3 (domain panel)
//! - `SensorDisplay`, `ActuatorDisplay`: T2-C (display projections)

use super::Panel;
use crate::chrome::Theme;
use crate::layout::Rect;
use crate::paint::DisplayCommand;
use crate::state::{Message, PanelId};
use crate::style::Color;

/// Tier: T2-C — Sensor data projected for display.
#[derive(Debug, Clone)]
pub struct SensorDisplay {
    /// Sensor name.
    pub name: String,
    /// Whether the sensor is active.
    pub active: bool,
    /// Alert count.
    pub alerts: usize,
}

/// Tier: T2-C — Actuator data projected for display.
#[derive(Debug, Clone)]
pub struct ActuatorDisplay {
    /// Actuator name.
    pub name: String,
    /// Whether the actuator is enabled.
    pub enabled: bool,
}

/// Tier: T3 — Guardian homeostasis loop monitor panel.
pub struct GuardianMonitorPanel {
    /// Sensor statuses.
    sensors: Vec<SensorDisplay>,
    /// Actuator statuses.
    actuators: Vec<ActuatorDisplay>,
    /// Current loop state (e.g. "running", "idle").
    loop_state: String,
    /// Current risk level (0.0 to 1.0).
    risk_level: f64,
}

impl GuardianMonitorPanel {
    /// Create a new guardian monitor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sensors: Vec::new(),
            actuators: Vec::new(),
            loop_state: "unknown".to_string(),
            risk_level: 0.0,
        }
    }

    /// Sync all guardian state from bridge results.
    pub fn sync(
        &mut self,
        sensors: Vec<SensorDisplay>,
        actuators: Vec<ActuatorDisplay>,
        loop_state: String,
        risk_level: f64,
    ) {
        self.sensors = sensors;
        self.actuators = actuators;
        self.loop_state = loop_state;
        self.risk_level = risk_level;
    }
}

impl Default for GuardianMonitorPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine color for a risk level.
fn risk_color(level: f64) -> Color {
    if level < 0.3 {
        return Theme::CONFIDENCE_UP; // green
    }
    if level >= 0.7 {
        return Theme::CONFIDENCE_DOWN; // red
    }
    Theme::STATUS_TEXT // gray
}

/// Paint the panel header with loop state.
fn paint_header(area: &Rect, loop_state: &str) -> Vec<DisplayCommand> {
    vec![
        DisplayCommand::DrawText {
            text: "Guardian — Homeostasis".to_string(),
            x: area.x + 16.0,
            y: area.y + 20.0,
            size: 14.0,
            color: Theme::SIDEBAR_ACTIVE,
            node_id: None,
        },
        DisplayCommand::DrawText {
            text: format!("Loop: {loop_state}"),
            x: area.x + 16.0,
            y: area.y + 42.0,
            size: Theme::FONT_SIZE_SMALL,
            color: Theme::STATUS_TEXT,
            node_id: None,
        },
    ]
}

/// Paint the risk indicator.
fn paint_risk(area: &Rect, risk_level: f64, y_offset: f32) -> DisplayCommand {
    let color = risk_color(risk_level);
    DisplayCommand::DrawText {
        text: format!("Risk: {:.0}%", risk_level * 100.0),
        x: area.x + 16.0,
        y: area.y + y_offset,
        size: Theme::FONT_SIZE,
        color,
        node_id: None,
    }
}

/// Paint a single sensor entry.
fn paint_sensor(area: &Rect, sensor: &SensorDisplay, y_offset: f32) -> DisplayCommand {
    let status = if sensor.active { "●" } else { "○" };
    let alert_text = if sensor.alerts > 0 {
        format!(" ({} alerts)", sensor.alerts)
    } else {
        String::new()
    };
    DisplayCommand::DrawText {
        text: format!("{status} {}{alert_text}", sensor.name),
        x: area.x + 24.0,
        y: area.y + y_offset,
        size: Theme::FONT_SIZE_SMALL,
        color: Theme::SIDEBAR_TEXT,
        node_id: None,
    }
}

/// Paint a single actuator entry.
fn paint_actuator(area: &Rect, actuator: &ActuatorDisplay, y_offset: f32) -> DisplayCommand {
    let status = if actuator.enabled { "ON" } else { "OFF" };
    DisplayCommand::DrawText {
        text: format!("{}: {status}", actuator.name),
        x: area.x + 24.0,
        y: area.y + y_offset,
        size: Theme::FONT_SIZE_SMALL,
        color: Theme::SIDEBAR_TEXT,
        node_id: None,
    }
}

impl Panel for GuardianMonitorPanel {
    fn id(&self) -> PanelId {
        PanelId::GUARDIAN
    }

    fn name(&self) -> &str {
        "Guardian Monitor"
    }

    fn paint(&self, area: Rect) -> Vec<DisplayCommand> {
        let mut cmds = paint_header(&area, &self.loop_state);
        cmds.push(paint_risk(&area, self.risk_level, 62.0));

        // Sensors section
        let mut y = 90.0;
        if !self.sensors.is_empty() {
            cmds.push(DisplayCommand::DrawText {
                text: "Sensors".to_string(),
                x: area.x + 16.0,
                y: area.y + y,
                size: Theme::FONT_SIZE,
                color: Theme::SIDEBAR_ACTIVE,
                node_id: None,
            });
            y += 20.0;
            for sensor in &self.sensors {
                cmds.push(paint_sensor(&area, sensor, y));
                y += 18.0;
            }
        }

        // Actuators section
        y += 10.0;
        if !self.actuators.is_empty() {
            cmds.push(DisplayCommand::DrawText {
                text: "Actuators".to_string(),
                x: area.x + 16.0,
                y: area.y + y,
                size: Theme::FONT_SIZE,
                color: Theme::SIDEBAR_ACTIVE,
                node_id: None,
            });
            y += 20.0;
            for actuator in &self.actuators {
                cmds.push(paint_actuator(&area, actuator, y));
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
    fn test_guardian_creation() {
        let panel = GuardianMonitorPanel::new();
        assert!(panel.sensors.is_empty());
        assert_eq!(panel.loop_state, "unknown");
        assert!(panel.risk_level.abs() < f64::EPSILON);
    }

    #[test]
    fn test_guardian_sync() {
        let mut panel = GuardianMonitorPanel::new();
        panel.sync(
            vec![SensorDisplay {
                name: "pamp".to_string(),
                active: true,
                alerts: 2,
            }],
            vec![ActuatorDisplay {
                name: "blocker".to_string(),
                enabled: true,
            }],
            "running".to_string(),
            0.45,
        );
        assert_eq!(panel.sensors.len(), 1);
        assert_eq!(panel.actuators.len(), 1);
        assert_eq!(panel.loop_state, "running");
    }

    #[test]
    fn test_risk_color_green() {
        let color = risk_color(0.1);
        assert_eq!(color.r, Theme::CONFIDENCE_UP.r);
    }

    #[test]
    fn test_risk_color_red() {
        let color = risk_color(0.8);
        assert_eq!(color.r, Theme::CONFIDENCE_DOWN.r);
    }

    #[test]
    fn test_guardian_paint_empty() {
        let panel = GuardianMonitorPanel::new();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 280.0,
            height: 600.0,
        };
        let cmds = panel.paint(area);
        // header(2) + risk(1) = 3
        assert_eq!(cmds.len(), 3);
    }
}
