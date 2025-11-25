// Event handler functions (button clicks, device refresh, write logic)

use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar};

/// Setup ISO file browser event
pub fn setup_iso_browser_event(
    iso_button: &Button,
    window: &ApplicationWindow,
    iso_entry: Entry,
    os_label: &Label,
    reset_advanced_options: impl Fn() + Clone + 'static,
) {
    let window_weak_browse = window.downgrade();
    let reset_advanced_options_clone = reset_advanced_options.clone();
    let os_label_clone = os_label.clone();

    iso_button.connect_clicked(move |_| {
        if let Some(window) = window_weak_browse.upgrade() {
            let dialog = FileChooserDialog::new(
                Some("Select ISO Image"),
                Some(&window),
                FileChooserAction::Open,
                &[ ]
            );
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
            let reset_advanced_options = reset_advanced_options_clone.clone();
            let os_label_clone2 = os_label_clone.clone();
            dialog.connect_response(move |dialog, resp| {
                if resp == gtk4::ResponseType::Ok {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        let path_str = file.to_string_lossy().to_string();
                        iso_entry_clone2.set_text(&path_str);
                        // Call the reusable reset logic
                        reset_advanced_options();

                        // Auto-detect OS type when ISO is selected
                        os_label_clone2.set_text("Detecting OS type...");
                        let detected = crate::utils::is_windows_iso(&path_str);
                        match detected {
                            Some(true) => os_label_clone2.set_text("Detected: Windows ISO"),
                            Some(false) => os_label_clone2.set_text("Detected: Linux ISO"),
                            None => os_label_clone2.set_text("Could not detect OS type"),
                        }
                    }
                }
                dialog.close();
            });
            dialog.show();
        }
    });
}

/// Setup USB device refresh event
pub fn setup_device_refresh_event(device_combo: &ComboBoxText, refresh_button: &Button) {
    let device_combo_clone = device_combo.clone();
    refresh_button.connect_clicked(move |_| {
        println!("[DEBUG] Refreshing USB device list...");
        device_combo_clone.remove_all();

        let devices = crate::utils::list_usb_devices();
        let device_count = devices.len();
        if devices.is_empty() {
            device_combo_clone.append_text("(No USB devices found)");
            device_combo_clone.set_active(Some(0));
        } else {
            for (path, description) in devices {
                device_combo_clone.append_text(&format!("{} - {}", path, description));
            }
            device_combo_clone.set_active(Some(0));
        }
        println!("[DEBUG] Found {} USB devices", device_count);
    });
}

/// Setup write button event
pub fn setup_write_button_event(
    write_button: &Button,
    iso_entry: &Entry,
    device_combo: &ComboBoxText,
    windows_group: &GtkBox,
    linux_group: &GtkBox,
    log_view: &TextView,
    progress_bar: &ProgressBar,
) {
    let write_button_clone = write_button.clone();
    let iso_entry_clone = iso_entry.clone();
    let device_combo_clone = device_combo.clone();
    let windows_group_clone = windows_group.clone();
    let linux_group_clone = linux_group.clone();
    let log_view_clone = log_view.clone();
    let progress_bar_clone = progress_bar.clone();

    write_button_clone.clone().connect_clicked(move |_| {
        let iso_path = iso_entry_clone.text().to_string();
        if iso_path.is_empty() {
            let buffer = log_view_clone.buffer();
            buffer.set_text("ERROR: No ISO file selected\n");
            return;
        }

        let active_device = device_combo_clone.active_text().unwrap_or_default();
        if active_device.is_empty() || active_device.contains("(refresh to list devices)") || active_device.contains("(No USB devices found)") {
            let buffer = log_view_clone.buffer();
            buffer.set_text("ERROR: No USB device selected or no devices found\n");
            return;
        }

        // Extract device path (before " - " separator)
        let device_path = active_device.split(" - ").next().unwrap_or("").trim();
        let device_path = device_path.to_string(); // Create owned copy
        if device_path.is_empty() {
            let buffer = log_view_clone.buffer();
            buffer.set_text("ERROR: Could not parse device path\n");
            return;
        }

        println!("[DEBUG] Starting USB write: ISO={}, Device={}", iso_path, device_path);

        // Update UI for write operation
        write_button_clone.set_sensitive(false);
        progress_bar_clone.set_fraction(0.0);
        progress_bar_clone.set_show_text(true);
        progress_bar_clone.set_text(Some("Starting..."));

        let buffer = log_view_clone.buffer();
        let mut log_text = format!("Starting write operation:\n");
        log_text.push_str(&format!("  ISO: {}\n", iso_path));
        log_text.push_str(&format!("  Device: {}\n", device_path));

        // Determine write mode and options
        let is_windows_mode = windows_group_clone.is_visible();

        if is_windows_mode {
            log_text.push_str("  Mode: Windows\n");
        } else if linux_group_clone.is_visible() {
            let persistence = linux_group_clone.first_child()
                .and_then(|child| child.downcast::<gtk4::CheckButton>().ok())
                .map_or(false, |cb| cb.is_active());
            log_text.push_str(&format!("  Mode: Linux (persistence: {})\n", if persistence { "enabled" } else { "disabled" }));
        }

        buffer.set_text(&log_text);

        // Show confirmation dialog before starting
        let dialog = gtk4::MessageDialog::builder()
            .text("Confirm USB Write Operation")
            .secondary_text(&format!("This will completely erase:\n{}\n\nProceed with write operation?", device_path))
            .buttons(gtk4::ButtonsType::OkCancel)
            .message_type(gtk4::MessageType::Warning)
            .build();

        let progress_bar_clone2 = progress_bar_clone.clone();
        let write_button_clone2 = write_button_clone.clone();
        let log_view_clone2 = log_view_clone.clone();
        let iso_path_clone = iso_path.clone();
        let device_path_clone = device_path.clone();

        dialog.connect_response(move |dialog, response| {
            dialog.close();

            if response != gtk4::ResponseType::Ok {
                write_button_clone2.set_sensitive(true);
                progress_bar_clone2.set_fraction(0.0);
                progress_bar_clone2.set_show_text(false);
                return;
            }

            let buffer = log_view_clone2.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let mut current_text = buffer.text(&start, &end, false).to_string();
            current_text.push_str("\n=== Starting write operation ===\n");
            buffer.set_text(&current_text);

            // Configure progress bar
            progress_bar_clone2.set_fraction(0.0);
            progress_bar_clone2.set_show_text(true);
            progress_bar_clone2.set_text(Some("Starting..."));
            progress_bar_clone2.set_visible(true);

            // Execute write operation with real-time progress
            let result = if is_windows_mode {
                println!("[DEBUG] Writing Windows ISO to USB");
                current_text.push_str("Starting Windows dual-partition write...\n");
                buffer.set_text(&current_text);
                crate::flows::windows_flow::write_windows_iso_to_usb(
                    &iso_path_clone,
                    &device_path_clone,
                    false,
                    &mut std::io::Cursor::new(Vec::new())
                )
            } else {
                println!("[DEBUG] Writing Linux ISO to USB");
                current_text.push_str("Starting Linux ISO write...\n");
                buffer.set_text(&current_text);
                crate::flows::linux_flow::write_iso_to_usb(
                    &iso_path_clone,
                    &device_path_clone,
                    &mut std::io::Cursor::new(Vec::new())
                )
            };

            // Update UI after operation completes
            progress_bar_clone2.set_fraction(1.0);
            progress_bar_clone2.set_text(Some("Complete!"));
            write_button_clone2.set_sensitive(true);

            let buffer = log_view_clone2.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let mut text = buffer.text(&start, &end, false).to_string();

            match result {
                Ok(()) => {
                    text.push_str("\n✓ Write operation completed successfully!\n");

                    // Show completion dialog
                    let completion_dialog = gtk4::MessageDialog::builder()
                        .text("USB creation complete!")
                        .message_type(gtk4::MessageType::Info)
                        .buttons(gtk4::ButtonsType::Ok)
                        .build();
                    completion_dialog.connect_response(|dialog, _| dialog.close());
                    completion_dialog.show();
                },
                Err(e) => {
                    text.push_str(&format!("\n✗ Write operation failed: {}\n", e));
                    progress_bar_clone2.set_text(Some("Failed"));
                }
            }

            buffer.set_text(&text);
        });

        dialog.show();
    });
}
