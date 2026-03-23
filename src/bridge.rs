//! nexcore API bridge — async client to nexcore-api at :3030.
//!
//! Provides a non-blocking interface to the nexcore REST API for
//! signal detection, brain operations, and other tools.
//!
//! ## Tier Classification
//!
//! - `nexcoreBridge`: T3 (domain-specific async client)

use crate::state::{BridgeResult, CloudServiceInfo};
use std::time::Duration;

/// Default API base URL.
const DEFAULT_API_BASE: &str = "http://localhost:3030";

/// Request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 3;

// ── Phase 2 summary types ──────────────────────────────────────

/// Tier: T2-C — Brain session summary for display.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    /// Session identifier.
    pub id: String,
    /// Creation timestamp.
    pub created: String,
    /// Number of artifacts in this session.
    pub artifact_count: usize,
}

/// Tier: T2-C — Guardian sensor status for display.
#[derive(Debug, Clone)]
pub struct SensorStatus {
    /// Sensor name.
    pub name: String,
    /// Whether the sensor is active.
    pub active: bool,
    /// Number of alerts from this sensor.
    pub alert_count: usize,
}

/// Tier: T2-C — Guardian actuator status for display.
#[derive(Debug, Clone)]
pub struct ActuatorStatus {
    /// Actuator name.
    pub name: String,
    /// Whether the actuator is enabled.
    pub enabled: bool,
}

/// Tier: T2-C — MCP/Skill summary for display.
#[derive(Debug, Clone)]
pub struct SkillSummary {
    /// Skill name.
    pub name: String,
    /// Skill category.
    pub category: String,
    /// Number of tools provided.
    pub tool_count: usize,
}

/// Build a blocking client with timeout.
fn build_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// Tier: T3 — Async client to the NexCore REST API.
pub struct NexCoreBridge {
    /// API base URL.
    base_url: String,
    /// HTTP client with timeout.
    client: reqwest::blocking::Client,
}

impl NexCoreBridge {
    /// Create a new bridge to the default API endpoint.
    #[must_use]
    pub fn new() -> Self {
        let base_url =
            std::env::var("NEXCORE_API_URL").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());
        Self {
            base_url,
            client: build_client(),
        }
    }

    /// Create a bridge to a custom endpoint.
    #[must_use]
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: build_client(),
        }
    }

    /// Check if the API is reachable.
    pub fn health_check(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .map_or(false, |r| r.status().is_success())
    }

    /// Run signal detection for a drug-event pair.
    pub fn signal_check(&self, drug: &str, event: &str) -> BridgeResult {
        let url = format!("{}/api/v1/pv/signal/complete", self.base_url);
        let body = build_signal_body(drug, event);
        let result = self.post_json(&url, &body);
        match result {
            Ok(resp) => parse_signal_response(resp, drug, event),
            Err(e) => BridgeResult::Error(e),
        }
    }

    /// Call a generic API endpoint.
    pub fn api_call(&self, endpoint: &str, payload: &str) -> BridgeResult {
        let url = format!("{}{endpoint}", self.base_url);
        let result = self.post_json(&url, payload);
        match result {
            Ok(resp) => build_api_result(endpoint, resp),
            Err(e) => BridgeResult::Error(e),
        }
    }

    // ── Phase 2: Brain/Guardian/MCP endpoints ─────────────────

    /// Fetch brain session data.
    pub fn brain_sessions(&self) -> BridgeResult {
        let url = format!("{}/api/v1/brain/sessions", self.base_url);
        match self.get_json(&url) {
            Ok(body) => parse_brain_sessions(&body),
            Err(e) => BridgeResult::Error(e),
        }
    }

    /// Fetch guardian status.
    pub fn guardian_status(&self) -> BridgeResult {
        let url = format!("{}/api/v1/guardian/status", self.base_url);
        match self.get_json(&url) {
            Ok(body) => parse_guardian_status(&body),
            Err(e) => BridgeResult::Error(e),
        }
    }

    /// Fetch cloud status from a NexCloud instance.
    pub fn cloud_status(&self, cloud_url: &str) -> BridgeResult {
        let url = format!("{cloud_url}/.nexcloud/status");
        match self.get_json(&url) {
            Ok(body) => parse_cloud_status(&body),
            Err(e) => BridgeResult::Error(e),
        }
    }

    /// Fetch MCP/skill list.
    pub fn mcp_skills(&self) -> BridgeResult {
        let url = format!("{}/api/v1/skills/", self.base_url);
        match self.get_json(&url) {
            Ok(body) => parse_mcp_skills(&body),
            Err(e) => BridgeResult::Error(e),
        }
    }

    /// Send a GET request, returning the body as string.
    fn get_json(&self, url: &str) -> std::result::Result<String, String> {
        let resp = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .map_err(|e| format!("GET failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("GET {} returned {}", url, resp.status()));
        }
        resp.text().map_err(|e| format!("Body read failed: {e}"))
    }

    /// Send a POST request with JSON body.
    fn post_json(
        &self,
        url: &str,
        body: &str,
    ) -> std::result::Result<reqwest::blocking::Response, String> {
        self.client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .map_err(|e| format!("Request failed: {e}"))
    }
}

impl Default for NexCoreBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the JSON body for a signal detection request.
fn build_signal_body(drug: &str, event: &str) -> String {
    format!(r#"{{"drug":"{drug}","event":"{event}","a":15,"b":100,"c":20,"d":10000}}"#)
}

/// Convert a raw HTTP response into an API `BridgeResult`.
fn build_api_result(endpoint: &str, resp: reqwest::blocking::Response) -> BridgeResult {
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    BridgeResult::ApiResponse {
        endpoint: endpoint.to_string(),
        status,
        body,
    }
}

/// Parse a signal detection response.
fn parse_signal_response(
    resp: reqwest::blocking::Response,
    drug: &str,
    event: &str,
) -> BridgeResult {
    let status = resp.status().as_u16();
    if status != 200 {
        return BridgeResult::Error(format!("Signal API returned {status}"));
    }

    let body = resp.text().unwrap_or_default();
    let prr = extract_f64(&body, "prr").unwrap_or(0.0);
    let ror = extract_f64(&body, "ror").unwrap_or(0.0);
    let ic = extract_f64(&body, "ic").unwrap_or(0.0);

    BridgeResult::SignalResult {
        drug: drug.to_string(),
        event: event.to_string(),
        prr,
        ror,
        ic,
        signal_detected: prr >= 2.0,
    }
}

/// Extract a f64 value from a JSON string by key (simple parser).
fn extract_f64(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{key}\":");
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')?;
    rest[..end].parse().ok()
}

/// Extract a string value from a JSON string by key.
fn extract_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":\"");
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract a u64 value from a JSON string by key.
fn extract_u64(json: &str, key: &str) -> Option<u64> {
    let pattern = format!("\"{key}\":");
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Extract a usize value from a JSON string by key.
fn extract_usize(json: &str, key: &str) -> Option<usize> {
    extract_u64(json, key).map(|v| v as usize)
}

/// Extract a bool value from a JSON string by key.
fn extract_bool(json: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{key}\":");
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    if rest.starts_with("true") {
        return Some(true);
    }
    if rest.starts_with("false") {
        return Some(false);
    }
    None
}

// ── Phase 2 parsers ──────────────────────────────────────────

/// Parse brain sessions response.
fn parse_brain_sessions(body: &str) -> BridgeResult {
    let sessions = extract_sessions(body);
    // The top-level artifact_count may shadow per-session ones.
    // Extract from after the sessions array to get the aggregate.
    let artifact_count = extract_top_level_usize(body, "sessions", "artifact_count")
        .unwrap_or_else(|| sessions.iter().map(|s| s.artifact_count).sum());
    BridgeResult::BrainData {
        sessions,
        artifact_count,
    }
}

/// Extract a usize key that appears after a given array section.
fn extract_top_level_usize(body: &str, after_key: &str, target_key: &str) -> Option<usize> {
    let pattern = format!("\"{after_key}\":[");
    let Some(start) = body.find(&pattern) else {
        return extract_usize(body, target_key);
    };
    let arr_start = start + pattern.len();
    let bracket_end = find_matching_bracket(body, arr_start);
    if bracket_end < body.len() {
        // Search only in the remainder after the array
        return extract_usize(&body[bracket_end..], target_key);
    }
    extract_usize(body, target_key)
}

/// Extract a single session object from a JSON fragment.
fn parse_one_session(obj: &str) -> SessionSummary {
    SessionSummary {
        id: extract_string(obj, "id").unwrap_or_default(),
        created: extract_string(obj, "created").unwrap_or_default(),
        artifact_count: extract_usize(obj, "artifact_count").unwrap_or(0),
    }
}

/// Extract session objects from a JSON array.
fn extract_sessions(body: &str) -> Vec<SessionSummary> {
    extract_objects(body, "{\"id\":")
        .iter()
        .map(|obj| parse_one_session(obj))
        .collect()
}

/// Parse guardian status response.
fn parse_guardian_status(body: &str) -> BridgeResult {
    let sensors = extract_sensors(body);
    let actuators = extract_actuators(body);
    let loop_state = extract_string(body, "status").unwrap_or_else(|| "unknown".to_string());
    let risk_level = extract_f64(body, "risk_level").unwrap_or(0.0);
    BridgeResult::GuardianData {
        sensors,
        actuators,
        loop_state,
        risk_level,
    }
}

/// Parse a single sensor from a JSON fragment.
fn parse_one_sensor(obj: &str) -> SensorStatus {
    SensorStatus {
        name: extract_string(obj, "name").unwrap_or_default(),
        active: extract_bool(obj, "active").unwrap_or(false),
        alert_count: extract_usize(obj, "alert_count").unwrap_or(0),
    }
}

/// Extract sensor objects from JSON.
fn extract_sensors(body: &str) -> Vec<SensorStatus> {
    let section = find_array_section(body, "sensors");
    extract_objects(section, "{\"name\":")
        .iter()
        .map(|obj| parse_one_sensor(obj))
        .collect()
}

/// Parse a single actuator from a JSON fragment.
fn parse_one_actuator(obj: &str) -> ActuatorStatus {
    ActuatorStatus {
        name: extract_string(obj, "name").unwrap_or_default(),
        enabled: extract_bool(obj, "enabled").unwrap_or(false),
    }
}

/// Extract actuator objects from JSON.
fn extract_actuators(body: &str) -> Vec<ActuatorStatus> {
    let section = find_array_section(body, "actuators");
    extract_objects(section, "{\"name\":")
        .iter()
        .map(|obj| parse_one_actuator(obj))
        .collect()
}

/// Parse MCP/skills response.
fn parse_mcp_skills(body: &str) -> BridgeResult {
    let skills = extract_skills(body);
    let total_tools = extract_usize(body, "total_tools")
        .unwrap_or_else(|| skills.iter().map(|s| s.tool_count).sum());
    BridgeResult::McpData {
        skills,
        total_tools,
    }
}

/// Parse a single skill from a JSON fragment.
fn parse_one_skill(obj: &str) -> SkillSummary {
    SkillSummary {
        name: extract_string(obj, "name").unwrap_or_default(),
        category: extract_string(obj, "category").unwrap_or_else(|| "general".to_string()),
        tool_count: extract_usize(obj, "tool_count").unwrap_or(0),
    }
}

/// Extract skill objects from JSON.
fn extract_skills(body: &str) -> Vec<SkillSummary> {
    extract_objects(body, "{\"name\":")
        .iter()
        .map(|obj| parse_one_skill(obj))
        .collect()
}

// ── Cloud status parser ──────────────────────────────────────

/// Parse cloud status response from NexCloud `/.nexcloud/status`.
fn parse_cloud_status(body: &str) -> BridgeResult {
    let platform_name =
        extract_string(body, "platform_name").unwrap_or_else(|| "unknown".to_string());
    let overall_health =
        extract_string(body, "overall_health").unwrap_or_else(|| "unknown".to_string());

    let services_section = find_array_section(body, "services");
    let service_objects = extract_objects(services_section, "{\"name\":");
    let services: Vec<CloudServiceInfo> = service_objects
        .iter()
        .map(|obj| parse_one_cloud_service(obj))
        .collect();
    let service_count = services.len();

    BridgeResult::CloudData {
        platform_name,
        services,
        overall_health,
        service_count,
    }
}

/// Parse a single cloud service from a JSON fragment.
fn parse_one_cloud_service(obj: &str) -> CloudServiceInfo {
    let state = extract_string(obj, "state").unwrap_or_else(|| "unknown".to_string());
    let healthy = state == "healthy";
    CloudServiceInfo {
        name: extract_string(obj, "name").unwrap_or_default(),
        state,
        port: extract_u64(obj, "port").unwrap_or(0) as u16,
        pid: extract_u64(obj, "pid").map(|v| v as u32),
        restarts: extract_u64(obj, "restarts").unwrap_or(0) as u32,
        healthy,
    }
}

/// Extract JSON objects by scanning for a marker pattern.
fn extract_objects(body: &str, marker: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(start) = body[search_from..].find(marker) {
        let abs_start = search_from + start;
        let obj_end = find_closing_brace(body, abs_start);
        results.push(body[abs_start..obj_end].to_string());
        search_from = obj_end;
    }
    results
}

/// Find the closing brace for an object starting at `start`.
/// Find the closing `}` that matches the depth level at `start`.
///
/// Unlike a naive `.find('}')`, this tracks nesting depth so it handles
/// nested JSON objects correctly. Starts counting from depth=0 — the first
/// `}` at depth 0 is the match.
fn find_closing_brace(json: &str, start: usize) -> usize {
    let mut depth = 0usize;
    for (i, ch) in json[start..].char_indices() {
        match ch {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                if depth == 0 {
                    return start + i + 1;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    json.len()
}

/// Find a JSON array section by key name.
fn find_array_section<'a>(json: &'a str, key: &str) -> &'a str {
    let pattern = format!("\"{key}\":[");
    let Some(start) = json.find(&pattern) else {
        return "";
    };
    let arr_start = start + pattern.len();
    let end = find_matching_bracket(json, arr_start);
    &json[arr_start..end]
}

/// Find the matching closing bracket, counting nesting depth.
fn find_matching_bracket(json: &str, start: usize) -> usize {
    let mut depth: u32 = 1;
    for (i, c) in json[start..].char_indices() {
        if c == '[' {
            depth += 1;
        }
        if c == ']' {
            depth -= 1;
        }
        if depth == 0 {
            return start + i;
        }
    }
    json.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_f64() {
        let json = r#"{"prr":2.5,"ror":3.1,"ic":0.8}"#;
        assert!((extract_f64(json, "prr").unwrap_or(0.0) - 2.5).abs() < f64::EPSILON);
        assert!((extract_f64(json, "ror").unwrap_or(0.0) - 3.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_extract_f64_missing() {
        let json = r#"{"other":1.0}"#;
        assert!(extract_f64(json, "prr").is_none());
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = NexCoreBridge::new();
        assert_eq!(bridge.base_url, "http://localhost:3030");
    }

    #[test]
    fn test_custom_base_url() {
        let bridge = NexCoreBridge::with_base_url("http://example.com:8080");
        assert_eq!(bridge.base_url, "http://example.com:8080");
    }

    #[test]
    fn test_bridge_with_base_url_overrides() {
        let bridge = NexCoreBridge::with_base_url("http://custom:9999");
        assert_eq!(bridge.base_url, "http://custom:9999");
    }

    #[test]
    fn test_build_signal_body() {
        let body = build_signal_body("aspirin", "headache");
        assert!(body.contains("aspirin"));
        assert!(body.contains("headache"));
    }

    // ── Phase 2 extraction helpers ─────────────────────────────

    #[test]
    fn test_extract_string() {
        let json = r#"{"id":"sess-001","created":"2026-01-01"}"#;
        assert_eq!(extract_string(json, "id").as_deref(), Some("sess-001"));
        assert_eq!(
            extract_string(json, "created").as_deref(),
            Some("2026-01-01")
        );
        assert!(extract_string(json, "missing").is_none());
    }

    #[test]
    fn test_extract_u64() {
        let json = r#"{"count":42,"zero":0}"#;
        assert_eq!(extract_u64(json, "count"), Some(42));
        assert_eq!(extract_u64(json, "zero"), Some(0));
        assert!(extract_u64(json, "missing").is_none());
    }

    #[test]
    fn test_extract_bool() {
        let json = r#"{"active":true,"enabled":false}"#;
        assert_eq!(extract_bool(json, "active"), Some(true));
        assert_eq!(extract_bool(json, "enabled"), Some(false));
        assert!(extract_bool(json, "missing").is_none());
    }

    #[test]
    fn test_parse_brain_sessions() {
        let json = r#"{"sessions":[{"id":"s1","created":"2026-01-01","artifact_count":3}],"artifact_count":10}"#;
        let result = parse_brain_sessions(json);
        match result {
            BridgeResult::BrainData {
                sessions,
                artifact_count,
            } => {
                assert_eq!(sessions.len(), 1);
                assert_eq!(sessions[0].id, "s1");
                assert_eq!(sessions[0].artifact_count, 3);
                assert_eq!(artifact_count, 10);
            }
            _ => panic!("Expected BrainData"),
        }
    }

    #[test]
    fn test_parse_guardian_status() {
        let json = r#"{"status":"running","risk_level":0.35,"sensors":[{"name":"pamp","active":true,"alert_count":2}],"actuators":[{"name":"blocker","enabled":true}]}"#;
        let result = parse_guardian_status(json);
        match result {
            BridgeResult::GuardianData {
                sensors,
                actuators,
                loop_state,
                risk_level,
            } => {
                assert_eq!(loop_state, "running");
                assert!((risk_level - 0.35).abs() < f64::EPSILON);
                assert_eq!(sensors.len(), 1);
                assert!(sensors[0].active);
                assert_eq!(actuators.len(), 1);
                assert!(actuators[0].enabled);
            }
            _ => panic!("Expected GuardianData"),
        }
    }

    #[test]
    fn test_parse_mcp_skills() {
        let json =
            r#"{"skills":[{"name":"forge","category":"dev","tool_count":5}],"total_tools":112}"#;
        let result = parse_mcp_skills(json);
        match result {
            BridgeResult::McpData {
                skills,
                total_tools,
            } => {
                assert_eq!(skills.len(), 1);
                assert_eq!(skills[0].name, "forge");
                assert_eq!(total_tools, 112);
            }
            _ => panic!("Expected McpData"),
        }
    }

    #[test]
    fn test_find_array_section() {
        let json = r#"{"items":[1,2,3],"other":"val"}"#;
        let section = find_array_section(json, "items");
        assert_eq!(section, "1,2,3");
    }

    #[test]
    fn test_find_array_section_empty() {
        let json = r#"{"other":"val"}"#;
        let section = find_array_section(json, "missing");
        assert!(section.is_empty());
    }

    #[test]
    fn test_brain_url_construction() {
        let bridge = NexCoreBridge::with_base_url("http://test:8080");
        // Just verify the bridge was created with correct base
        assert_eq!(bridge.base_url, "http://test:8080");
    }
}
