use dioxus::prelude::LaunchBuilder;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use gtk::prelude::GtkSettingsExt;
use muxspace::components::app as App;
use muxspace::runtime_env;
use tracing::info;

fn main() {
    runtime_env::configure_runtime_modules();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    gtk::init().expect("GTK initialization failed");
    if let Some(settings) = gtk::Settings::default() {
        let configured_dpi = settings.gtk_xft_dpi();
        settings.set_gtk_xft_dpi(96 * 1024);
        info!(
            configured_dpi,
            "Normalized GTK Xft DPI before WebKit startup"
        );
    }

    info!(
        gio_modules = ?runtime_env::gio_modules_path(),
        "Configured desktop runtime modules"
    );

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
                html { width: 100%; height: 100%; font-size: 100%; }
                body, #main { width: 100%; height: 100%; }
                body { margin: 0; padding: 0; overflow: hidden; }
                .app {
                    position: absolute;
                    inset: 0;
                    display: flex;
                    min-width: 0;
                    min-height: 0;
                }
                .sidebar {
                    flex: 0 0 16.25rem;
                    min-width: 16.25rem;
                    height: 100%;
                }
                .main-content {
                    flex: 1 1 auto;
                    min-width: 0;
                    min-height: 0;
                }
                .content-area {
                    min-width: 0;
                    min-height: 0;
                }
                button:focus-visible, input:focus-visible, textarea:focus-visible {
                    outline: 0.125rem solid #818cf8;
                    outline-offset: 0.125rem;
                }
                @media (max-width: 56.25rem) {
                    .sidebar {
                        flex-basis: 13.75rem;
                        min-width: 13.75rem;
                    }
                }
            </style>"#
                .to_string(),
        );

    LaunchBuilder::desktop().with_cfg(config).launch(App);
}
