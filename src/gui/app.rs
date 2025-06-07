use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Label};

pub fn run_gui() {
    let app = Application::builder()
        .application_id("com.example.usbbootablecreator")
        .build();

    app.connect_activate(|app| {
        // Check for required packages before showing the main window
        if let Some((_, install_cmd)) = crate::utils::check_required_packages() {
            let dialog = gtk4::Dialog::with_buttons(
                Some("Missing Required Packages"),
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                &[("OK", gtk4::ResponseType::Ok)],
            );
            let content = dialog.content_area();
            let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
            let label = Label::new(Some("Some required system packages are missing. Please install them using the command below:"));
            vbox.append(&label);
            let text_area = gtk4::TextView::new();
            text_area.set_editable(false);
            text_area.set_cursor_visible(false);
            text_area.buffer().set_text(&install_cmd);
            vbox.append(&text_area);
            let copy_button = Button::with_label("Copy Command");
            let install_cmd_clone = install_cmd.clone();
            copy_button.connect_clicked(move |_| {
                if let Some(display) = gtk4::gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&install_cmd_clone);
                }
            });
            vbox.append(&copy_button);
            content.append(&vbox);
            dialog.set_modal(true);
            dialog.set_default_response(gtk4::ResponseType::Ok);
            dialog.connect_response(|dialog, _| dialog.close());
            dialog.show();
        } else {
            let window = ApplicationWindow::builder()
                .application(app)
                .title("MajUSB Bootable Creator")
                .default_width(830)
                .default_height(400)
                .resizable(true)
                .build();
            window.show();
        }
    });

    app.run();
}
