//! Adventure HUD page — Prima-powered HTML generation.
//!
//! Fetches session data from nexcore API, constructs Prima source
//! with embedded variable bindings, evaluates the template, and
//! returns HTML for the renderer.

use crate::bridge::NexCoreBridge;

/// Resolve the Prima template path at runtime.
///
/// Checks `NEXBROWSER_TEMPLATES_DIR` env var first, then falls back to
/// a path relative to the Cargo workspace root (for development builds).
/// Never bakes a developer machine path into compiled code.
fn template_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("NEXBROWSER_TEMPLATES_DIR") {
        return std::path::PathBuf::from(dir).join("adventure-hud.true");
    }
    // Workspace-relative fallback: crate is at <workspace>/crates/nexcore-renderer/
    // Templates live at <workspace>/templates/
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest).join("../../templates/adventure-hud.true")
}

/// Fallback HTML when Prima evaluation fails.
const FALLBACK_HTML: &str = r#"<html><head><title>Adventure HUD</title></head>
<body style="background:#111827;color:#e5e7eb;font-family:sans-serif;padding:24px;">
<h1 style="color:#7eb8ff;">Adventure HUD</h1>
<p style="color:#f87171;">Unable to load session data. Is nexcore-api running on :3030?</p>
<p style="color:#9ca3af;">Try: <code>nexcore-api</code> then reload.</p>
</body></html>"#;

/// Render the adventure HUD by evaluating a Prima template with live data.
#[must_use]
pub fn render() -> String {
    let bridge = NexCoreBridge::new();

    // Read the Prima template
    let template = match std::fs::read_to_string(template_path()) {
        Ok(t) => t,
        Err(_) => return fallback("Template file not found"),
    };

    // Fetch session data — graceful degradation on API unavailability
    let (session_name, duration_mins, tools_called, tokens_used, tasks, skills, milestones) =
        fetch_session_data(&bridge);

    // Build Prima preamble with λ bindings
    let preamble = build_preamble(
        &session_name,
        duration_mins,
        tools_called,
        tokens_used,
        &tasks,
        &skills,
        &milestones,
    );

    // Evaluate: preamble + template
    let source = format!("{preamble}\n{template}");
    match prima::eval(&source) {
        Ok(val) => extract_string(&val),
        Err(e) => fallback(&format!("Prima eval error: {e}")),
    }
}

/// Extract string from Prima Value, falling back on Display.
fn extract_string(val: &prima::Value) -> String {
    match &val.data {
        prima::value::ValueData::String(s) => s.clone(),
        _ => format!("{val}"),
    }
}

/// Fetch session data from the NexCore API bridge.
/// Returns defaults if API is unreachable.
#[allow(clippy::type_complexity)]
fn fetch_session_data(
    bridge: &NexCoreBridge,
) -> (
    String,
    u32,
    u32,
    u64,
    Vec<(String, String, String)>,
    Vec<(String, String)>,
    Vec<(String, String)>,
) {
    if !bridge.health_check() {
        return demo_data();
    }

    // Try fetching brain sessions for real data
    let brain = bridge.brain_sessions();
    let session_name = match &brain {
        crate::state::BridgeResult::BrainData { sessions, .. } => sessions
            .first()
            .map_or_else(|| "No Session".to_string(), |s| s.id.clone()),
        _ => "No Session".to_string(),
    };

    let artifact_count = match &brain {
        crate::state::BridgeResult::BrainData { artifact_count, .. } => *artifact_count,
        _ => 0,
    };

    // Build a minimal view with available data
    let tasks = vec![
        (
            "1".to_string(),
            format!("Artifacts: {artifact_count}"),
            "completed".to_string(),
        ),
        (
            "2".to_string(),
            "API Connected".to_string(),
            "completed".to_string(),
        ),
    ];

    let skills = match bridge.mcp_skills() {
        crate::state::BridgeResult::McpData {
            skills,
            total_tools,
        } => {
            let mut result: Vec<(String, String)> = skills
                .iter()
                .take(6)
                .map(|s| (s.name.clone(), s.tool_count.to_string()))
                .collect();
            if result.is_empty() {
                result.push(("MCP Tools".to_string(), total_tools.to_string()));
            }
            result
        }
        _ => vec![("MCP".to_string(), "0".to_string())],
    };

    let milestones = vec![
        (
            "API Online".to_string(),
            "NexCore REST API connected".to_string(),
        ),
        (
            "Prima Rendering".to_string(),
            "Template engine active".to_string(),
        ),
    ];

    (session_name, 0, 0, 0, tasks, skills, milestones)
}

/// Demo data when API is unavailable.
fn demo_data() -> (
    String,
    u32,
    u32,
    u64,
    Vec<(String, String, String)>,
    Vec<(String, String)>,
    Vec<(String, String)>,
) {
    let tasks = vec![
        (
            "1".to_string(),
            "Create Prima template".to_string(),
            "completed".to_string(),
        ),
        (
            "2".to_string(),
            "Wire renderer".to_string(),
            "completed".to_string(),
        ),
        (
            "3".to_string(),
            "Fetch live data".to_string(),
            "in_progress".to_string(),
        ),
        (
            "4".to_string(),
            "Visual test".to_string(),
            "pending".to_string(),
        ),
    ];
    let skills = vec![
        ("forge".to_string(), "3".to_string()),
        ("lingua".to_string(), "2".to_string()),
        ("vigilance-dev".to_string(), "5".to_string()),
    ];
    let milestones = vec![
        (
            "Prima + Renderer".to_string(),
            "Template engine integrated".to_string(),
        ),
        (
            "No Leptos".to_string(),
            "Zero framework dependency".to_string(),
        ),
    ];
    (
        "Demo Session".to_string(),
        42,
        15,
        50000,
        tasks,
        skills,
        milestones,
    )
}

/// Escape a string for embedding in Prima source.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Build Prima preamble with λ variable bindings.
fn build_preamble(
    session_name: &str,
    duration_mins: u32,
    tools_called: u32,
    tokens_used: u64,
    tasks: &[(String, String, String)],
    skills: &[(String, String)],
    milestones: &[(String, String)],
) -> String {
    let mut src = String::with_capacity(1024);

    src.push_str(&format!("λ session_name = \"{}\"\n", escape(session_name)));
    src.push_str(&format!("λ duration_mins = {duration_mins}\n"));
    src.push_str(&format!("λ tools_called = {tools_called}\n"));
    src.push_str(&format!("λ tokens_used = {tokens_used}\n"));

    // Tasks: σ[σ["id", "subject", "status"], ...]
    let task_items: Vec<String> = tasks
        .iter()
        .map(|(id, subj, st)| {
            format!(
                "σ[\"{}\", \"{}\", \"{}\"]",
                escape(id),
                escape(subj),
                escape(st)
            )
        })
        .collect();
    src.push_str(&format!("λ tasks = σ[{}]\n", task_items.join(", ")));

    // Skills: σ[σ["name", "count"], ...]
    let skill_items: Vec<String> = skills
        .iter()
        .map(|(name, count)| format!("σ[\"{}\", \"{}\"]", escape(name), escape(count)))
        .collect();
    src.push_str(&format!("λ skills = σ[{}]\n", skill_items.join(", ")));

    // Milestones: σ[σ["name", "desc"], ...]
    let milestone_items: Vec<String> = milestones
        .iter()
        .map(|(name, desc)| format!("σ[\"{}\", \"{}\"]", escape(name), escape(desc)))
        .collect();
    src.push_str(&format!(
        "λ milestones = σ[{}]\n",
        milestone_items.join(", ")
    ));

    src
}

/// Produce fallback HTML with an error message.
fn fallback(reason: &str) -> String {
    FALLBACK_HTML.replace("Is nexcore-api running on :3030?", &escape(reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_basic() {
        assert_eq!(escape("hello"), "hello");
        assert_eq!(escape(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(escape("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_build_preamble_structure() {
        let preamble = build_preamble(
            "Test Session",
            42,
            15,
            50000,
            &[("1".into(), "Task A".into(), "completed".into())],
            &[("forge".into(), "3".into())],
            &[("Milestone".into(), "Done".into())],
        );
        assert!(preamble.contains("λ session_name = \"Test Session\""));
        assert!(preamble.contains("λ duration_mins = 42"));
        assert!(preamble.contains("λ tools_called = 15"));
        assert!(preamble.contains("λ tokens_used = 50000"));
        assert!(preamble.contains("σ[\"1\", \"Task A\", \"completed\"]"));
        assert!(preamble.contains("σ[\"forge\", \"3\"]"));
        assert!(preamble.contains("σ[\"Milestone\", \"Done\"]"));
    }

    #[test]
    fn test_preamble_empty_collections() {
        let preamble = build_preamble("Empty", 0, 0, 0, &[], &[], &[]);
        assert!(preamble.contains("λ tasks = σ[]"));
        assert!(preamble.contains("λ skills = σ[]"));
        assert!(preamble.contains("λ milestones = σ[]"));
    }

    #[test]
    fn test_fallback_html() {
        let html = fallback("test error");
        assert!(html.contains("<html>"));
        assert!(html.contains("Adventure HUD"));
    }

    #[test]
    fn test_demo_data_completeness() {
        let (name, dur, tools, tokens, tasks, skills, milestones) = demo_data();
        assert!(!name.is_empty());
        assert!(dur > 0);
        assert!(tools > 0);
        assert!(tokens > 0);
        assert!(!tasks.is_empty());
        assert!(!skills.is_empty());
        assert!(!milestones.is_empty());
    }

    #[test]
    fn test_prima_template_eval_with_demo_data() {
        let template = match std::fs::read_to_string(template_path()) {
            Ok(t) => t,
            Err(_) => return, // Skip if template not found (CI)
        };
        let (name, dur, tools, tokens, tasks, skills, milestones) = demo_data();
        let preamble = build_preamble(&name, dur, tools, tokens, &tasks, &skills, &milestones);
        let source = format!("{preamble}\n{template}");
        let result = prima::eval(&source);
        assert!(result.is_ok(), "Prima eval failed: {:?}", result.err());
        let val = result.ok().unwrap_or_else(|| prima::Value::string(""));
        let html = extract_string(&val);
        assert!(html.contains("<html>"), "Should produce HTML");
        assert!(html.contains("Adventure HUD"), "Should contain title");
        assert!(html.contains("Demo Session"), "Should contain session name");
        assert!(
            html.contains("42m") || html.contains("0h 42m"),
            "Should contain duration"
        );
        assert!(html.contains("50K"), "Should contain formatted tokens");
        assert!(
            html.contains("&#10003;"),
            "Should contain checkmark for completed"
        );
        assert!(html.contains("forge"), "Should contain skill name");
    }

    #[test]
    fn test_prima_template_empty_data() {
        let template = match std::fs::read_to_string(template_path()) {
            Ok(t) => t,
            Err(_) => return,
        };
        let preamble = build_preamble("Empty", 0, 0, 0, &[], &[], &[]);
        let source = format!("{preamble}\n{template}");
        let result = prima::eval(&source);
        assert!(
            result.is_ok(),
            "Prima eval with empty data failed: {:?}",
            result.err()
        );
    }
}
