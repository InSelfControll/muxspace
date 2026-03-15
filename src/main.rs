use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use tracing::info;

mod browser;
mod components;
pub mod hotkeys;
mod pty;
mod state;
mod sync;

use components::app as App;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Muxspace Dioxus...");

    let window = WindowBuilder::new()
        .with_title("Muxspace")
        .with_decorations(false)
        .with_inner_size(LogicalSize::new(1400.0, 900.0))
        .with_min_inner_size(LogicalSize::new(800.0, 600.0));

    let config = Config::default()
        .with_window(window)
        .with_menu(None)
        .with_custom_head(
            r#"<style>
                * { box-sizing: border-box; }
                body { margin: 0; padding: 0; overflow: hidden; }
            </style>"#
                .to_string(),
        );

    LaunchBuilder::desktop().with_cfg(config).launch(App);
}
