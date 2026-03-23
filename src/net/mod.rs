//! Network module - fetches resources.

mod adventure;

use crate::{Error, Result};
use url::Url;

/// Fetch HTML content from a URL.
///
/// # Errors
/// Returns error if URL is invalid or fetch fails.
pub fn fetch(url_str: &str) -> Result<String> {
    let url = Url::parse(url_str)?;
    match url.scheme() {
        "file" => {
            let path = url
                .to_file_path()
                .map_err(|()| Error::Parse("Invalid file path".into()))?;
            std::fs::read_to_string(path).map_err(Error::Io)
        }
        "http" | "https" => {
            let response = reqwest::blocking::get(url_str)?;
            Ok(response.text()?)
        }
        "data" => parse_data_url(url_str),
        "nex" => Ok(nex_page(url.host_str().unwrap_or("welcome"))),
        _ => Err(Error::Parse(format!(
            "Unsupported scheme: {}",
            url.scheme()
        ))),
    }
}

fn parse_data_url(url: &str) -> Result<String> {
    let parts: Vec<&str> = url.splitn(2, ',').collect();
    if parts.len() == 2 {
        Ok(parts[1].to_string())
    } else {
        Err(Error::Parse("Invalid data URL".into()))
    }
}

/// Fetch binary content from a URL.
///
/// Supports `file://`, `http://`, and `https://` schemes.
/// Returns raw bytes suitable for image decoding.
///
/// # Errors
/// Returns error if URL is invalid, scheme is unsupported, or fetch fails.
pub fn fetch_bytes(url_str: &str) -> Result<Vec<u8>> {
    let url = Url::parse(url_str)?;
    match url.scheme() {
        "file" => {
            let path = url
                .to_file_path()
                .map_err(|()| Error::Parse("Invalid file path".into()))?;
            std::fs::read(path).map_err(Error::Io)
        }
        "http" | "https" => {
            let response = reqwest::blocking::get(url_str)?;
            Ok(response.bytes()?.to_vec())
        }
        _ => Err(Error::Parse(format!(
            "Unsupported scheme for binary fetch: {}",
            url.scheme()
        ))),
    }
}

/// Generate HTML for a `nex://` internal page.
#[must_use]
fn nex_page(page: &str) -> String {
    match page {
        "adventure" => adventure::render(),
        "welcome" => nex_welcome(),
        "grounded" => nex_grounded(),
        "hypothesis" => nex_internal("Hypothesis Queue", "View and manage hypotheses."),
        "experience" => nex_internal("Experience Store", "Browse structured learnings."),
        "signal" => nex_internal("Signal Dashboard", "PV signal detection interface."),
        "brain" => nex_internal("Brain View", "Working memory and artifacts."),
        "guardian" => nex_internal("Guardian Monitor", "Homeostasis control loop."),
        "cloud" => nex_cloud(),
        "mcp" => nex_internal("MCP Tools", "Model Context Protocol tool explorer."),
        "settings" => nex_internal("Settings", "NexBrowser configuration."),
        _ => nex_internal("Not Found", &format!("Unknown page: nex://{page}")),
    }
}

/// Welcome page HTML.
fn nex_welcome() -> String {
    r#"<html><head><title>NexBrowser - GROUNDED AI Collaborator</title></head>
<body style="background-color: #0f0f1a; color: #c8c8e0;">
<h1 style="color: #7eb8ff;">NexBrowser</h1>
<p style="color: #556688;">The GROUNDED loop made visible.</p>
<h2 style="color: #9ecbff;">AI reasoning meets reality</h2>
<p>Where outcomes are observable, learnings persist, and trust is earned.</p>
<p style="color: #66ddaa;">nex://grounded</p><p>GROUNDED Loop Monitor</p>
<p style="color: #66ddaa;">nex://signal</p><p>Signal Detection Dashboard</p>
<p style="color: #66ddaa;">nex://experience</p><p>Experience Store Browser</p>
<p style="color: #66ddaa;">nex://hypothesis</p><p>Hypothesis Queue</p>
<p style="color: #66ddaa;">nex://cloud</p><p>Cloud Status Dashboard</p>
<p style="color: #334455;">NexVigilant - Powered by the Vigilance Kernel</p>
</body></html>"#
        .to_string()
}

/// GROUNDED loop monitor page HTML.
fn nex_grounded() -> String {
    r#"<html><head><title>GROUNDED Loop Monitor</title></head>
<body style="background-color: #0f0f1a; color: #c8c8e0;">
<h1 style="color: #7eb8ff;">GROUNDED Loop</h1>
<p style="color: #66ddaa;">reason -> test -> observe -> integrate -> persist</p>
<h2 style="color: #9ecbff;">Active Cycle</h2>
<p>Use the sidebar panels to interact with the GROUNDED loop.</p>
<p>Propose hypotheses, approve experiments, and review learnings.</p>
<h2 style="color: #9ecbff;">Principles</h2>
<p style="color: #ffcc66;">Outputs have observable consequences</p>
<p style="color: #ffcc66;">Observations persist and accumulate</p>
<p style="color: #ffcc66;">AI can initiate, not just respond</p>
<p style="color: #ffcc66;">Verification is native</p>
</body></html>"#
        .to_string()
}

/// Cloud status page HTML.
fn nex_cloud() -> String {
    r#"<html><head><title>Cloud Status</title></head>
<body style="background-color: #0f0f1a; color: #c8c8e0;">
<h1 style="color: #7eb8ff;">Cloud Status</h1>
<p style="color: #66ddaa;">NexCloud service monitoring dashboard.</p>
<h2 style="color: #9ecbff;">Services</h2>
<p>Use the sidebar Cloud Status panel to view live service health,</p>
<p>restart counts, and overall platform status from NexCloud.</p>
<h2 style="color: #9ecbff;">Endpoint</h2>
<p style="color: #ffcc66;">GET /.nexcloud/status</p>
<p>Returns JSON with platform_name, services, overall_health, and resource_snapshot.</p>
</body></html>"#
        .to_string()
}

/// Generic internal page HTML.
fn nex_internal(title: &str, description: &str) -> String {
    format!(
        r#"<html><head><title>{title}</title></head>
<body style="background-color: #0f0f1a; color: #c8c8e0;">
<h1 style="color: #7eb8ff;">{title}</h1>
<p>{description}</p>
</body></html>"#
    )
}

/// Resolve a relative URL against a base.
#[must_use]
pub fn resolve(base: &str, relative: &str) -> Option<String> {
    let base_url = Url::parse(base).ok()?;
    let resolved = base_url.join(relative).ok()?;
    Some(resolved.to_string())
}
