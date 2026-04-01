mod app;
mod cipher;
mod core;
mod gui;
mod history;
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
