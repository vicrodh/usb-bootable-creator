use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView};

pub fn run_gui() {
    let app = Application::builder()
        .application_id("com.example.usbbootablecreator")
        .build();

    app.connect_activate(|app| {
        // Main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Rust USB Bootable Creator")
            .default_width(500)
            .default_height(400)
            .build();

        // Main vertical box
        let vbox = GtkBox::new(Orientation::Vertical, 12);
        vbox.set_margin_top(16);
        vbox.set_margin_bottom(16);
        vbox.set_margin_start(16);
        vbox.set_margin_end(16);

        // ISO selection
        let iso_hbox = GtkBox::new(Orientation::Horizontal, 8);
        let iso_entry = Entry::builder().placeholder_text("Select ISO file...").build();
        let iso_button = Button::with_label("Browse");
        iso_hbox.append(&iso_entry);
        iso_hbox.append(&iso_button);
        vbox.append(&Label::new(Some("ISO Image:")));
        vbox.append(&iso_hbox);

        // Label to show detected OS type
        let os_label = Label::new(None);
        vbox.append(&os_label);

        // USB device selection
        let device_combo = ComboBoxText::new();
        device_combo.append_text("(refresh to list devices)");
        let refresh_button = Button::with_label("Refresh Devices");
        let device_hbox = GtkBox::new(Orientation::Horizontal, 8);
        device_hbox.append(&device_combo);
        device_hbox.append(&refresh_button);
        vbox.append(&Label::new(Some("USB Device:")));
        vbox.append(&device_hbox);

        // Cluster size (optional, can be hidden or a combo)
        let cluster_entry = Entry::builder().placeholder_text("Cluster size (optional)").build();
        vbox.append(&Label::new(Some("Cluster Size:")));
        vbox.append(&cluster_entry);

        // Write button
        let write_button = Button::with_label("Write to USB");
        vbox.append(&write_button);

        // Log area
        let log_label = Label::new(Some("Log:"));
        let log_view = TextView::new();
        log_view.set_editable(false);
        let log_scroll = ScrolledWindow::builder().min_content_height(100).child(&log_view).build();
        vbox.append(&log_label);
        vbox.append(&log_scroll);

        // --- Event handlers ---
        // ISO browse (no detection here)
        let iso_entry_clone = iso_entry.clone();
        let window_weak = window.downgrade();
        iso_button.connect_clicked(move |_| {
            if let Some(window) = window_weak.upgrade() {
                let dialog = FileChooserDialog::new(
                    Some("Select ISO Image"),
                    Some(&window),
                    FileChooserAction::Open,
                    &[ ("Open", gtk4::ResponseType::Ok), ("Cancel", gtk4::ResponseType::Cancel) ],
                );
                let filter = FileFilter::new();
                filter.add_pattern("*.iso");
                filter.set_name(Some("ISO files"));
                dialog.add_filter(&filter);
                let iso_entry_clone2 = iso_entry_clone.clone();
                dialog.connect_response(move |dialog, resp| {
                    if resp == gtk4::ResponseType::Ok {
                        if let Some(file) = dialog.file().and_then(|f| f.path()) {
                            let path_str = file.to_string_lossy().to_string();
                            iso_entry_clone2.set_text(&path_str);
                        }
                    }
                    dialog.close();
                });
                dialog.show();
            }
        });

        // Refresh devices (now uses real backend)
        let device_combo_clone = device_combo.clone();
        refresh_button.connect_clicked(move |_| {
            device_combo_clone.remove_all();
            let devices = crate::utils::list_usb_devices();
            if devices.is_empty() {
                device_combo_clone.append_text("No USB devices found");
            } else {
                for (dev, label) in devices {
                    device_combo_clone.append_text(&format!("{} ({})", dev, label));
                }
            }
        });

        // Write button (call cli_helper via pkexec for detection + write)
        let log_view_clone = log_view.clone();
        let iso_entry_clone = iso_entry.clone();
        let device_combo_clone = device_combo.clone();
        let os_label_clone = os_label.clone();
        write_button.connect_clicked(move |_| {
            let buffer = log_view_clone.buffer();
            let iso_path = iso_entry_clone.text();
            let device_text = device_combo_clone.active_text().map(|s| s.to_string()).unwrap_or_default();
            let usb_device = device_text.split_whitespace().next().unwrap_or("");
            if iso_path.is_empty() || usb_device.is_empty() {
                buffer.set_text("Please select an ISO and a USB device.");
                return;
            }
            buffer.set_text("Requesting elevated permissions and writing to USB...\n");
            // Use the absolute path to cli_helper
            let cli_helper_path = "./target/debug/cli_helper";
            use std::process::Command;
            let output = Command::new("pkexec")
                .arg(cli_helper_path)
                .arg(&iso_path)
                .arg(usb_device)
                .output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    buffer.set_text(&format!("{}{}", stdout, stderr));
                    if stdout.contains("Windows ISO") {
                        os_label_clone.set_text("Detected: Windows ISO");
                    } else if stdout.contains("Linux ISO") {
                        os_label_clone.set_text("Detected: Linux ISO");
                    }
                },
                Err(e) => {
                    buffer.set_text(&format!("Failed to launch helper: {}", e));
                }
            }
        });

        window.set_child(Some(&vbox));
        window.show();
    });

    app.run();
}
