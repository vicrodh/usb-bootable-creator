// Dialog creation functions (missing packages, confirmation, completion)

use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Dialog, MessageDialog, ButtonsType, MessageType, ResponseType,
            Button, Box as GtkBox, Label, TextView, Orientation, FileChooserAction,
            FileChooserDialog, FileFilter, Entry, Window};
use glib::MainContext;
use crate::services::{OsCategory, mock_list_os_by_category};

/// Show missing packages dialog with installation command
pub fn show_missing_packages_dialog_simple(
    parent: Option<&ApplicationWindow>,
    install_cmd: String,
) {
    let dialog = Dialog::with_buttons(
        Some("Missing Required Packages"),
        parent,
        gtk4::DialogFlags::MODAL,
        &[("OK", gtk4::ResponseType::Ok)],
    );
    dialog.set_default_width(640);
    let content = dialog.content_area();
    let vbox = GtkBox::new(Orientation::Vertical, 8);
    let label = Label::new(Some("Some required system packages are missing. Please install them using the command below:"));
    vbox.append(&label);
    let text_area = TextView::new();
    text_area.set_editable(false);
    text_area.set_cursor_visible(false);
    text_area.buffer().set_text(&install_cmd);
    vbox.append(&text_area);
    let copy_button = Button::with_label("Copy Command");
    copy_button.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&install_cmd);
        }
    });
    vbox.append(&copy_button);
    content.append(&vbox);
    dialog.set_modal(true);
    dialog.set_default_response(gtk4::ResponseType::Ok);
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}

/// Show confirmation dialog for USB write operation (exact app.rs implementation)
pub fn show_usb_write_confirmation_dialog(
    parent: Option<&ApplicationWindow>,
    device_path: &str,
) -> gtk4::MessageDialog {
    let dialog = gtk4::MessageDialog::builder()
        .text("Confirm USB Write Operation")
        .secondary_text(&format!("This will completely erase:\n{}\n\nProceed with write operation?", device_path))
        .buttons(gtk4::ButtonsType::OkCancel)
        .message_type(gtk4::MessageType::Warning)
        .build();
    dialog.set_default_width(640);

    if let Some(p) = parent {
        dialog.set_transient_for(Some(p));
    }

    dialog
}

/// Show completion dialog after successful USB creation
pub fn show_usb_completion_dialog() -> gtk4::MessageDialog {
    let dialog = gtk4::MessageDialog::builder()
        .text("USB creation complete!")
        .message_type(gtk4::MessageType::Info)
        .buttons(gtk4::ButtonsType::Ok)
        .build();
    dialog.set_default_width(640);
    dialog
}

/// Show Flatpak permissions instructions dialog
pub fn show_flatpak_instructions_dialog(window: &ApplicationWindow) -> gtk4::MessageDialog {
    let dialog = MessageDialog::builder()
        .text("🔒 Permisos de Root Requeridos")
        .secondary_text(
            "Esta aplicación necesita acceso root para gestionar dispositivos USB.\n\n\
            ⚠️  ESTÁS EJECUTANDO EN FLATPAK ⚠️\n\n\
            En Flatpak no se pueden obtener permisos automáticamente.\n\
            Por favor, cierra esta aplicación y ejecute:\n\n\
            💻 COMANDO RECOMENDADO:\n\
            flatpak-spawn --host pkexec flatpak run com.github.vicrodh.MajUSB\n\n\
            📋 INSTRUCCIONES:\n\
            1. Instale flatpak-xdg-utils si no lo tiene:\n\
               flatpak install flathub org.freedesktop.Sdk.Extension.flatpak-xdg-utils\n\n\
            2. Ejecute el comando recomendado arriba\n\n\
            3. O instale manualmente las herramientas necesarias"
        )
        .buttons(ButtonsType::Ok)
        .message_type(MessageType::Warning)
        .modal(true)
        .transient_for(window)
        .build();

    dialog.set_default_width(640);
    dialog.set_default_response(ResponseType::Ok);
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    dialog.show();
    dialog
}

/// Show ISO file chooser dialog (exact app.rs implementation)
pub fn show_iso_file_chooser_dialog_app(
    parent: &ApplicationWindow,
    iso_entry: &Entry,
    os_label: &Label,
    reset_advanced_options: impl Fn() + 'static,
) {
    let dialog = FileChooserDialog::new(
        Some("Select ISO Image"),
        Some(parent),
        FileChooserAction::Open,
        &[ ]
    );
    dialog.set_default_width(640);
    dialog.add_button("Open", gtk4::ResponseType::Ok);
    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
    let filter = FileFilter::new();
    filter.add_pattern("*.iso");
    filter.set_name(Some("ISO files"));
    dialog.add_filter(&filter);

    // Set initial folder to user's home directory
    let user_home = crate::utils::get_user_home();
    let gfile = gtk4::gio::File::for_path(&user_home);
    let _ = dialog.set_current_folder(Some(&gfile));

    let iso_entry_clone2 = iso_entry.clone();
    let os_label_clone = os_label.clone();
    dialog.connect_response(move |dialog, resp| {
        if resp == gtk4::ResponseType::Ok {
            if let Some(file) = dialog.file().and_then(|f| f.path()) {
                let path_str = file.to_string_lossy().to_string();
                iso_entry_clone2.set_text(&path_str);
                // Call the reusable reset logic
                reset_advanced_options();

                // Auto-detect OS type when ISO is selected
                os_label_clone.set_text("Detecting OS type...");
                let detected = crate::utils::is_windows_iso(&path_str);
                match detected {
                    Some(true) => os_label_clone.set_text("Detected: Windows ISO"),
                    Some(false) => os_label_clone.set_text("Detected: Linux ISO"),
                    None => os_label_clone.set_text("Could not detect OS type"),
                }
            }
        }
        dialog.close();
    });
    dialog.show();
}

/// Warning dialog for direct dd mode with Windows ISOs
pub fn show_dd_mode_warning_dialog(parent: &ApplicationWindow) -> bool {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(MessageType::Warning)
        .buttons(ButtonsType::YesNo)
        .text("Direct dd mode is NOT recommended for Windows 10/11")
        .secondary_text(
            "This mode writes the ISO directly without creating the required GPT dual-partition layout (FAT32 BOOT + NTFS ESD-USB).\n\n\
             Consequences:\n\
             • May fail to boot on UEFI systems\n\
             • Issues with files >4GB on FAT32-only layouts\n\
             • Not equivalent to Media Creation Tool behavior\n\n\
             Recommended: Use the default dual-partition mode.\n\n\
             Reference: Microsoft UEFI/GPT guidance\n\
             https://learn.microsoft.com/windows-hardware/manufacture/desktop/create-uefi-based-hard-drive-partitions"
        )
        .build();

    dialog.set_default_width(640);
    let response = MainContext::default().block_on(dialog.run_future());
    dialog.close();
    response == ResponseType::Yes
}

/// ISO Downloader Dialog - POC implementation
pub fn show_iso_downloader_dialog(parent: Option<&ApplicationWindow>) {
    let window = Window::new();
    window.set_title(Some("ISO Downloader"));
    window.set_default_size(800, 600);
    window.set_modal(true);

    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    let main_box = GtkBox::new(Orientation::Vertical, 10);
    main_box.set_margin_top(10);
    main_box.set_margin_bottom(10);
    main_box.set_margin_start(10);
    main_box.set_margin_end(10);

    // Title
    let title_label = Label::builder()
        .label("📀 ISO Downloader")
        .css_classes(["title-1"])
        .build();
    main_box.append(&title_label);

    // Category selection box
    let category_box = GtkBox::new(Orientation::Horizontal, 10);
    let category_label = Label::new(Some("Select category:"));

    let linux_button = Button::with_label("🐧 Linux");
    let windows_button = Button::with_label("🪟 Windows");

    category_box.append(&category_label);
    category_box.append(&linux_button);
    category_box.append(&windows_button);
    main_box.append(&category_box);

    // Results log text view
    let results_label = Label::new(Some("API Results:"));
    main_box.append(&results_label);

    let log_textview = TextView::new();
    log_textview.set_editable(false);
    log_textview.set_wrap_mode(gtk4::WrapMode::Word);
    log_textview.set_vexpand(true);

    let scrolled_window = gtk4::ScrolledWindow::builder()
        .child(&log_textview)
        .min_content_height(300)
        .build();
    main_box.append(&scrolled_window);

    // Close button
    let close_button = Button::with_label("Close");
    main_box.append(&close_button);

    window.set_child(Some(&main_box));

    // Button connections
    let log_buffer = log_textview.buffer();
    let window_clone = window.clone();

    linux_button.connect_clicked(move |_| {
        let buffer = log_buffer.clone();
        append_to_log(&buffer, "Fetching Linux distributions...\n");

        let response = mock_list_os_by_category(OsCategory::Linux);

        append_to_log(&buffer, &format!("✅ Success: {}\n", response.message.unwrap_or_default()));

        for os in &response.data {
            append_to_log(&buffer, &format!(
                "📦 {} {}\n   ID: {}\n   Size: {}MB\n   {}\n\n",
                os.name,
                os.version,
                os.id,
                os.size_mb.unwrap_or(0),
                os.description.as_deref().unwrap_or_default()
            ));
        }

        if response.data.is_empty() {
            append_to_log(&buffer, "❌ No Linux distributions found\n");
        }
    });

    let log_buffer_windows = log_textview.buffer();
    windows_button.connect_clicked(move |_| {
        let buffer = log_buffer_windows.clone();
        append_to_log(&buffer, "Fetching Windows versions...\n");

        let response = mock_list_os_by_category(OsCategory::Windows);

        append_to_log(&buffer, &format!("✅ Success: {}\n", response.message.unwrap_or_default()));

        for os in &response.data {
            append_to_log(&buffer, &format!(
                "📦 {} {}\n   ID: {}\n   Size: {}MB\n   {}\n\n",
                os.name,
                os.version,
                os.id,
                os.size_mb.unwrap_or(0),
                os.description.as_deref().unwrap_or_default()
            ));
        }

        if response.data.is_empty() {
            append_to_log(&buffer, "❌ No Windows versions found\n");
        }
    });

    close_button.connect_clicked(move |_| {
        window_clone.close();
    });

    window.show();
}

/// Helper function to append text to text buffer
fn append_to_log(buffer: &gtk4::TextBuffer, text: &str) {
    let mut end_iter = buffer.end_iter();
    buffer.insert(&mut end_iter, text);
}
