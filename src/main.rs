//! NexBrowser - 100% Rust browser.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting NexBrowser");

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("data:text/html,{}", nexcore_renderer::DEFAULT_PAGE));

    if let Err(e) = nexcore_renderer::window::run(&url) {
        tracing::error!("Browser failed: {e}");
        std::process::exit(1);
    }
}
