//! # NexVigilant Core — Renderer
//!
//! A 100% Rust browser rendering engine built from scratch.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
//!
//! ## Architecture
//!
//! ```text
//! URL → [Net] → HTML → [DOM] → DOM Tree
//!                               ↓
//!              CSS → [Style] → Style Tree
//!                               ↓
//!                    [Layout] → Layout Tree
//!                               ↓
//!                    [Paint] → Display List → GPU
//! ```
//!
//! ## Tier Classification
//!
//! - **T1**: `Node`, `Rect`, `Color` (universal primitives)
//! - **T2-P**: `StyledNode`, `LayoutBox` (cross-domain)
//! - **T3**: `Browser`, `Tab`, `DevTools` (domain-specific)

pub mod app;
pub mod bridge;
pub mod chrome;
pub mod content;
pub mod debug_server;
pub mod dom;
pub mod gpu;
pub mod grounded;
pub mod grounding;
pub mod input;
pub mod layout;
pub mod net;
pub mod paint;
pub mod panels;
pub mod scroll;
pub mod state;
pub mod style;
pub mod text;
pub mod visual_primitives;
pub mod window;

pub use app::Browser;

/// Default welcome page HTML — grounded to Lex Primitiva color language.
///
/// Each section is colored by its dominant primitive:
/// - Title: σ Sequence (Electric Blue) — the rendering pipeline
/// - Shortcuts: μ Mapping (Amber) — key-to-action mappings
/// - Libraries: ρ Recursion (Teal) — foundational recursive tools
/// - Pipeline: Σ Sum (Coral) — aggregation of stages
pub const DEFAULT_PAGE: &str = r#"<html>
<head><title>NexBrowser - Welcome</title></head>
<body style="background-color: #0D0D14; color: #B0A89C;">

<h1 style="color: #4FC3F7;">NexBrowser v0.1.0</h1>
<p style="color: #8C8478;">100% Rust. Zero WebView. Pure GPU rendering.</p>

<h2 style="color: #4FC3F7;">Getting Started</h2>
<p style="color: #D4CCC0;">Type a URL in the address bar above and press Enter to navigate.</p>
<p style="color: #D4CCC0;">Try any website, a local file:// path, or a data: URL.</p>

<h2 style="color: #FFB74D;">Keyboard Shortcuts</h2>
<p style="color: #FFB74D;">Ctrl+L</p>
<p>Focus the address bar</p>
<p style="color: #FFB74D;">Enter</p>
<p>Navigate to the typed URL</p>
<p style="color: #FFB74D;">Escape</p>
<p>Clear / unfocus address bar</p>
<p style="color: #FFB74D;">Ctrl+R / F5</p>
<p>Reload current page</p>
<p style="color: #FFB74D;">Ctrl+Plus / Ctrl+Minus</p>
<p>Zoom in / Zoom out</p>
<p style="color: #FFB74D;">Ctrl+0</p>
<p>Reset zoom to 100%</p>
<p style="color: #FFB74D;">F12</p>
<p>Developer tools (coming soon)</p>

<h2 style="color: #F06292;">Rendering Pipeline</h2>
<p style="color: #D4CCC0;">URL -> Net -> HTML -> DOM Tree</p>
<p style="color: #D4CCC0;">CSS -> Style -> Styled Tree -> Layout -> Display List -> GPU</p>

<h2 style="color: #4DB6AC;">Built With</h2>
<p style="color: #4DB6AC;">wgpu</p>
<p>GPU abstraction layer</p>
<p style="color: #4DB6AC;">winit</p>
<p>Cross-platform windowing</p>
<p style="color: #4DB6AC;">html5ever</p>
<p>Spec-compliant HTML parser (from Servo)</p>
<p style="color: #4DB6AC;">cosmic-text</p>
<p>Font shaping and glyph rasterization</p>
<p style="color: #4DB6AC;">taffy</p>
<p>Flexbox and CSS Grid layout engine</p>

<h2 style="color: #78909C;">Lex Primitiva</h2>
<p style="color: #4FC3F7;">σ Sequence</p>
<p style="color: #FFB74D;">μ Mapping</p>
<p style="color: #CE93D8;">ς State</p>
<p style="color: #4DB6AC;">ρ Recursion</p>
<p style="color: #78909C;">∅ Void</p>
<p style="color: #EF5350;">∂ Boundary</p>
<p style="color: #9CCC65;">ν Frequency</p>
<p style="color: #ECEFF1;">∃ Existence</p>
<p style="color: #FFD54F;">π Persistence</p>
<p style="color: #FF8A65;">→ Causality</p>
<p style="color: #4DD0E1;">κ Comparison</p>
<p style="color: #66BB6A;">N Quantity</p>
<p style="color: #7986CB;">λ Location</p>
<p style="color: #C62828;">∝ Irreversibility</p>
<p style="color: #F06292;">Σ Sum</p>
<p style="color: #FFAB91;">× Product</p>

<p style="color: #5C5448;">NexVigilant / NexCore - Powered by the Vigilance Kernel</p>

</body>
</html>"#;

/// Visual shapes demo — each shape colored by its dominant primitive.
///
/// Demonstrates Circle, Triangle, Line rendering via DisplayCommand.
pub const SHAPES_DEMO: &str = r#"<html>
<head><title>NexBrowser - Visual Primitives</title></head>
<body style="background-color: #0D0D14; color: #B0A89C;">

<h1 style="color: #4FC3F7;">Visual Primitives</h1>
<p style="color: #8C8478;">Shape primitives grounded to T1 Lex Primitiva</p>

<h2 style="color: #7986CB;">Circle (λ + N + μ)</h2>
<p style="color: #D4CCC0;">center_location + radius_quantity + color_mapping</p>

<h2 style="color: #4FC3F7;">Triangle (σ[λ,λ,λ] + μ)</h2>
<p style="color: #D4CCC0;">sequence_of_3_locations + color_mapping</p>

<h2 style="color: #FF8A65;">Line (λ → λ + N)</h2>
<p style="color: #D4CCC0;">start → end causality + width</p>

<h2 style="color: #F06292;">Rectangle (Σ + N + N)</h2>
<p style="color: #D4CCC0;">aggregate_of_4_vertices + width + height</p>

<p style="color: #78909C;">TODO: Canvas element for dynamic shape rendering</p>
<p style="color: #5C5448;">Prima Integration: Shapes → T1 Primitives → Pattern Matching</p>

</body>
</html>"#;

/// Result type for renderer operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Renderer error types.
#[derive(Debug, nexcore_error::Error)]
pub enum Error {
    /// Network error during fetch.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// HTML parsing error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Layout error.
    #[error("Layout error: {0}")]
    Layout(String),

    /// GPU/rendering error.
    #[error("Render error: {0}")]
    Render(String),

    /// URL parsing error.
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
