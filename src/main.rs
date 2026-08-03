mod config;
mod engine;
mod profiles;
mod ui;

use relm4::RelmApp;
use ui::AppModel;

fn main() {
    let _ = relm4::main_application();
    let app = RelmApp::new("org.humanized.autoclicker");
    app.run::<AppModel>(());
}
