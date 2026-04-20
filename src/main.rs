mod app;
mod carrier;
mod cipher;
mod config;
mod core;
mod gui;
mod github_sync;
mod history;
mod keys;
mod models;
mod secure;
mod ui;

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|arg| arg == "--tui") {
        app::SighFarApp::default().run()
    } else {
        gui::launch_gui()
    }
}
