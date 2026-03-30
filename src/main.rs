mod app;
mod cipher;
mod history;
mod models;
mod secure;
mod ui;

fn main() -> anyhow::Result<()> {
    app::SighFarApp::default().run()
}
