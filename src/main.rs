use libadwaita::{
    Application, ApplicationWindow,
    gio::prelude::{ApplicationExt, ApplicationExtManual},
    gtk::prelude::GtkWindowExt,
};

mod host;
mod ui;

fn main() {
    let application = Application::builder()
        .application_id("com.zendard.quickscreen")
        .build();

    application.connect_activate(|app| {
        let content = ui::build_home();
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Quickscreen")
            .default_width(350)
            .content(&content)
            .build();
        window.present();
    });
    application.run();
}
