// Dialog creation functions (missing packages, confirmation, completion)

use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Dialog, MessageDialog, ButtonsType, MessageType, ResponseType};

/// Show missing packages dialog with installation command
pub fn show_missing_packages_dialog(
    parent: Option<&ApplicationWindow>,
    missing_packages: &[String],
    install_cmd: &str,
) {
    let dialog = Dialog::with_buttons(
        Some("Missing Required Packages"),
        parent,
        gtk4::DialogFlags::MODAL,
        &[("OK", ResponseType::Ok)],
    );
    let content = dialog.content_area();
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let label = gtk4::Label::new(Some(&format!(
        "Some required system packages are missing ({} packages):",
        missing_packages.len()
    )));
    vbox.append(&label);

    let packages_text = gtk4::Label::new(Some(&missing_packages.join(", ")));
    packages_text.set_wrap(true);
    packages_text.set_max_width_chars(50);
    vbox.append(&packages_text);

    let install_label = gtk4::Label::new(Some("Install with:"));
    vbox.append(&install_label);

    let text_area = gtk4::TextView::new();
    text_area.set_editable(false);
    text_area.set_cursor_visible(false);
    text_area.buffer().set_text(install_cmd);
    vbox.append(&text_area);

    let copy_button = gtk4::Button::with_label("Copy Command");
    let install_cmd_clone = install_cmd.to_string();
    copy_button.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&install_cmd_clone);
        }
    });
    vbox.append(&copy_button);

    content.append(&vbox);
    dialog.set_modal(true);
    dialog.set_default_response(ResponseType::Ok);
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}

/// Show confirmation dialog for USB write operation
pub fn show_confirmation_dialog(
    parent: Option<&ApplicationWindow>,
    device_name: &str,
) -> bool {
    let dialog = gtk4::MessageDialog::builder()
        .text("Confirm USB Overwrite")
        .secondary_text(&format!("All data on {} will be erased!\n\nAre you sure you want to continue?", device_name))
        .buttons(ButtonsType::OkCancel)
        .message_type(MessageType::Warning)
        .build();

    if let Some(p) = parent {
        dialog.set_transient_for(Some(p));
    }

    dialog.set_default_response(ResponseType::Cancel);

    // Simple blocking implementation for now
    let (sender, receiver) = std::sync::mpsc::channel();
    let dialog_clone = dialog.clone();
    dialog.connect_response(move |_, response| {
        let _ = sender.send(response);
        dialog_clone.close();
    });

    dialog.show();
    matches!(receiver.recv(), Ok(ResponseType::Ok))
}

/// Show completion dialog after successful USB creation
pub fn show_completion_dialog(
    _parent: Option<&ApplicationWindow>,
) {
    println!("✓ USB creation complete!");
}

/// Show error dialog
pub fn show_error_dialog(
    _parent: Option<&ApplicationWindow>,
    title: &str,
    message: &str,
) {
    println!("ERROR - {}: {}", title, message);
}

/// Show progress dialog with progress bar
pub fn show_progress_dialog(
    _parent: Option<&ApplicationWindow>,
    _title: &str,
) -> (gtk4::Dialog, gtk4::ProgressBar, gtk4::Label) {
    // TODO: Implement proper progress dialog
    // For now, return minimal dialog
    let dialog = gtk4::Dialog::new();

    if let Some(p) = _parent {
        dialog.set_transient_for(Some(p));
    }
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    let status_label = gtk4::Label::new(None);
    let progress_bar = gtk4::ProgressBar::new();

    dialog.content_area().append(&vbox);
    vbox.append(&status_label);
    vbox.append(&progress_bar);

    (dialog, progress_bar, status_label)
}
