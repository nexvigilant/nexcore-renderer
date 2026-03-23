//! Debug HTTP server for NexBrowser frame capture.
//!
//! Serves the latest rendered frame as a PNG at `http://localhost:9333/`.
//! Used by Chrome DevTools MCP to view and analyze NexBrowser output.
//!
//! ## Tier Classification
//!
//! - `DebugServer`: T3 (domain-specific debug infrastructure)
//! - `FrameBridge`: T2-C (σ Sequence + μ Mapping — channel-based frame transfer)

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Port for the debug HTTP server.
pub const DEBUG_PORT: u16 = 9333;

/// Shared state between the HTTP server and the render loop.
///
/// The HTTP thread requests a frame capture, the render loop fulfills it.
///
/// Tier: T2-C (σ Sequence + μ Mapping)
pub struct FrameBridge {
    /// Signals that a frame capture is needed.
    requested: Mutex<bool>,
    /// Wakes the render loop when a request arrives.
    request_notify: Condvar,
    /// The captured PNG data.
    frame_data: Mutex<Option<Vec<u8>>>,
    /// Wakes the HTTP thread when frame data is ready.
    ready_notify: Condvar,
}

impl FrameBridge {
    /// Create a new frame bridge.
    fn new() -> Self {
        Self {
            requested: Mutex::new(false),
            request_notify: Condvar::new(),
            frame_data: Mutex::new(None),
            ready_notify: Condvar::new(),
        }
    }

    /// Called by the HTTP server: request a frame and block until it arrives.
    fn request_frame(&self) -> Option<Vec<u8>> {
        // Signal that we want a frame
        if let Ok(mut req) = self.requested.lock() {
            *req = true;
        }
        self.request_notify.notify_one();

        // Wait for the frame data (with timeout to avoid deadlock)
        if let Ok(guard) = self.frame_data.lock() {
            let result = self
                .ready_notify
                .wait_timeout(guard, std::time::Duration::from_secs(5));
            if let Ok((data, _timeout)) = result {
                return data.clone();
            }
        }
        None
    }

    /// Called by the render loop: check if a frame was requested.
    pub fn check_requested(&self) -> bool {
        self.requested.lock().map(|req| *req).unwrap_or(false)
    }

    /// Called by the render loop: deliver the captured frame.
    pub fn deliver_frame(&self, png_data: Vec<u8>) {
        if let Ok(mut req) = self.requested.lock() {
            *req = false;
        }
        if let Ok(mut data) = self.frame_data.lock() {
            *data = Some(png_data);
        }
        self.ready_notify.notify_all();
    }
}

/// Start the debug server and return the shared frame bridge.
///
/// The server runs on a background thread listening on `127.0.0.1:9333`.
/// Returns `None` if the port is already in use (non-fatal).
pub fn start() -> Option<Arc<FrameBridge>> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEBUG_PORT);

    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("Debug server failed to bind port {port}: {e}");
            return None;
        }
    };

    tracing::info!("Debug server listening on http://localhost:{port}/");

    let bridge = Arc::new(FrameBridge::new());
    let bridge_clone = Arc::clone(&bridge);

    thread::Builder::new()
        .name("debug-server".into())
        .spawn(move || {
            run_server(listener, bridge_clone);
        })
        .ok();

    Some(bridge)
}

/// Server loop: accept connections and serve frames.
fn run_server(listener: TcpListener, bridge: Arc<FrameBridge>) {
    for stream in listener.incoming().flatten() {
        handle_connection(stream, &bridge);
    }
}

/// Handle a single HTTP connection.
fn handle_connection(mut stream: std::net::TcpStream, bridge: &FrameBridge) {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    if n == 0 {
        return;
    }

    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the request line to check path
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    match path {
        "/favicon.ico" => {
            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(response.as_bytes()).ok();
        }
        "/health" => {
            let body = r#"{"status":"ok","server":"nexbrowser-debug"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).ok();
        }
        _ => {
            // Default: serve the latest frame as PNG (or HTML wrapper)
            if path == "/raw" || path == "/screenshot.png" {
                serve_raw_png(&mut stream, bridge);
            } else {
                serve_html_frame(&mut stream, bridge, path);
            }
        }
    }
}

/// Serve the raw PNG bytes.
fn serve_raw_png(stream: &mut std::net::TcpStream, bridge: &FrameBridge) {
    match bridge.request_frame() {
        Some(png_data) => {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n",
                png_data.len()
            );
            stream.write_all(header.as_bytes()).ok();
            stream.write_all(&png_data).ok();
        }
        None => {
            let body = "Screenshot capture timed out";
            let response = format!(
                "HTTP/1.1 504 Gateway Timeout\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).ok();
        }
    }
}

/// Serve an HTML page that embeds the screenshot with auto-refresh.
fn serve_html_frame(stream: &mut std::net::TcpStream, bridge: &FrameBridge, _path: &str) {
    match bridge.request_frame() {
        Some(png_data) => {
            let mut base64_out = Vec::new();
            // Manual base64 encode (no external dep)
            base64_encode(&png_data, &mut base64_out);
            let b64 = String::from_utf8_lossy(&base64_out);

            let html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
<title>NexBrowser Debug View</title>
<meta http-equiv="refresh" content="3">
<style>
body {{ background: #0a0a14; margin: 0; display: flex; flex-direction: column; align-items: center; font-family: monospace; color: #7eb8ff; }}
h1 {{ margin: 10px 0 5px; font-size: 16px; }}
p {{ margin: 2px; font-size: 12px; color: #556688; }}
img {{ border: 1px solid #334455; max-width: 100vw; max-height: 90vh; }}
</style>
</head>
<body>
<h1>NexBrowser Live View</h1>
<p>Auto-refreshes every 3 seconds. <a href="/screenshot.png" style="color:#66ddaa">Raw PNG</a></p>
<img src="data:image/png;base64,{b64}" alt="NexBrowser frame"/>
</body>
</html>"#
            );

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache\r\n\r\n{}",
                html.len(),
                html
            );
            stream.write_all(response.as_bytes()).ok();
        }
        None => {
            let body = "<html><body style='background:#0a0a14;color:#ff6666;font-family:monospace'><h1>Capture Timeout</h1><p>NexBrowser render loop did not respond in 5s.</p></body></html>";
            let response = format!(
                "HTTP/1.1 504 Gateway Timeout\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).ok();
        }
    }
}

// ── Base64 encoder (zero-dep) ───────────────────────────────────

const B64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode bytes to base64 (RFC 4648).
fn base64_encode(input: &[u8], output: &mut Vec<u8>) {
    let mut i = 0;
    let len = input.len();
    while i + 2 < len {
        let n =
            (u32::from(input[i]) << 16) | (u32::from(input[i + 1]) << 8) | u32::from(input[i + 2]);
        output.push(B64_CHARS[((n >> 18) & 0x3F) as usize]);
        output.push(B64_CHARS[((n >> 12) & 0x3F) as usize]);
        output.push(B64_CHARS[((n >> 6) & 0x3F) as usize]);
        output.push(B64_CHARS[(n & 0x3F) as usize]);
        i += 3;
    }
    let remaining = len - i;
    if remaining == 2 {
        let n = (u32::from(input[i]) << 16) | (u32::from(input[i + 1]) << 8);
        output.push(B64_CHARS[((n >> 18) & 0x3F) as usize]);
        output.push(B64_CHARS[((n >> 12) & 0x3F) as usize]);
        output.push(B64_CHARS[((n >> 6) & 0x3F) as usize]);
        output.push(b'=');
    } else if remaining == 1 {
        let n = u32::from(input[i]) << 16;
        output.push(B64_CHARS[((n >> 18) & 0x3F) as usize]);
        output.push(B64_CHARS[((n >> 12) & 0x3F) as usize]);
        output.push(b'=');
        output.push(b'=');
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_empty() {
        let mut out = Vec::new();
        base64_encode(&[], &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn test_base64_encode_hello() {
        let mut out = Vec::new();
        base64_encode(b"Hello", &mut out);
        assert_eq!(String::from_utf8_lossy(&out), "SGVsbG8=");
    }

    #[test]
    fn test_base64_encode_three_bytes() {
        let mut out = Vec::new();
        base64_encode(b"Man", &mut out);
        assert_eq!(String::from_utf8_lossy(&out), "TWFu");
    }

    #[test]
    fn test_base64_encode_one_byte() {
        let mut out = Vec::new();
        base64_encode(b"M", &mut out);
        assert_eq!(String::from_utf8_lossy(&out), "TQ==");
    }

    #[test]
    fn test_base64_encode_two_bytes() {
        let mut out = Vec::new();
        base64_encode(b"Ma", &mut out);
        assert_eq!(String::from_utf8_lossy(&out), "TWE=");
    }

    #[test]
    fn test_debug_port_default() {
        assert_eq!(DEBUG_PORT, 9333);
    }

    #[test]
    fn test_frame_bridge_default_state() {
        let bridge = FrameBridge::new();
        assert!(!bridge.check_requested());
    }

    #[test]
    fn test_frame_bridge_deliver_clears_request() {
        let bridge = FrameBridge::new();
        // Manually set requested
        if let Ok(mut req) = bridge.requested.lock() {
            *req = true;
        }
        assert!(bridge.check_requested());
        bridge.deliver_frame(vec![1, 2, 3]);
        assert!(!bridge.check_requested());
    }
}
