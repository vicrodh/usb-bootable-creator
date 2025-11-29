use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, TextView, ProgressBar, MessageDialog, ButtonsType, MessageType};
use glib::{self, Priority};
use std::io;

use crate::flows::linux_persistence::{self, PersistenceConfig, PartitionTableType};
use crate::gui::widgets as gui_widgets;
use crate::gui::dialogs as gui_dialogs;

enum WorkerMessage {
    Log(String),
    Status(String),
    Done(Result<(), String>),
}

/// Writer that forwards log output to the GUI channel.
struct ChannelWriter {
    sender: glib::Sender<WorkerMessage>,
}

impl std::io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).to_string();
        let _ = self.sender.send(WorkerMessage::Log(msg));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}



pub fn run_gui(needs_root: bool, is_flatpak: bool) {
    // Apply user's visual theme settings before creating GUI
    crate::utils::apply_user_theme();

    let app = Application::builder()
        .application_id("com.github.vicrodh.MajUSB")
        .build();

    app.connect_activate(move |app| {
        // Check for required/optional packages before showing the main window.
        if let Some(pkg) = crate::utils::check_required_packages_split() {
            // Build a simple window instead of a dialog to avoid transient issues before main UI.
            let missing_win = ApplicationWindow::builder()
                .application(app)
                .title("Missing Packages")
                .default_width(720)
                .default_height(420)
                .resizable(true)
                .build();

            let vbox = GtkBox::new(Orientation::Vertical, 12);
            vbox.set_margin_top(16);
            vbox.set_margin_bottom(16);
            vbox.set_margin_start(16);
            vbox.set_margin_end(16);

            let intro = Label::new(Some("Some packages are missing to allow this app to work properly."));
            intro.set_wrap(true);
            vbox.append(&intro);

            let mut text = String::new();
            if !pkg.missing_required.is_empty() {
                text.push_str("# Required packages (Linux USB creation)\n");
                if let Some(cmd) = &pkg.install_cmd_required {
                    text.push_str(cmd);
                    text.push('\n');
                }
                text.push('\n');
            } else {
                text.push_str("# Required packages\n# All required packages are installed.\n\n");
            }

            text.push_str("# Optional packages (Windows / persistence)\n");
            if !pkg.missing_optional.is_empty() {
                if let Some(cmd) = &pkg.install_cmd_optional {
                    text.push_str(cmd);
                    text.push('\n');
                }
            } else {
                text.push_str("# All optional packages are installed.\n");
            }

            let text_view = TextView::new();
            text_view.set_editable(false);
            text_view.set_cursor_visible(false);
            text_view.set_monospace(true);
            text_view.buffer().set_text(&text);
            text_view.set_wrap_mode(gtk4::WrapMode::Word);

            let scroll = gtk4::ScrolledWindow::builder()
                .min_content_height(200)
                .child(&text_view)
                .build();
            vbox.append(&scroll);

            let copy_button = Button::with_label("Copy to clipboard");
            {
                let text_view_clone = text_view.clone();
                copy_button.connect_clicked(move |_| {
                    if let Some(display) = gtk4::gdk::Display::default() {
                        let clipboard = display.clipboard();
                        let buffer = text_view_clone.buffer();
                        let start = buffer.start_iter();
                        let end = buffer.end_iter();
                        let content = buffer.text(&start, &end, false).to_string();
                        clipboard.set_text(&content);
                    }
                });
            }
            vbox.append(&copy_button);

            let ok_button = Button::with_label("OK");
            {
                let missing_required = !pkg.missing_required.is_empty();
                let app_clone = app.clone();
                let missing_win_clone = missing_win.clone();
                ok_button.connect_clicked(move |_| {
                    if missing_required {
                        // Required packages missing: notify and exit.
                        let app_quit = app_clone.clone();
                        let dialog = MessageDialog::builder()
                            .transient_for(&missing_win_clone)
                            .modal(true)
                            .message_type(MessageType::Warning)
                            .buttons(ButtonsType::Ok)
                            .text("Please install the required packages and restart the application.")
                            .build();
                        dialog.run_async(move |d, _| {
                            d.close();
                            app_quit.quit();
                        });
                    } else {
                        missing_win_clone.close();
                    }
                });
            }
            vbox.append(&ok_button);

            missing_win.set_child(Some(&vbox));
            missing_win.show();

            // If required packages are missing, do not continue to main window.
            if !pkg.missing_required.is_empty() {
                return;
            }
        }

        {
            // Main window
            let window = ApplicationWindow::builder()
                .application(app)
                .title("MajUSB Bootable Creator")
                .default_width(830)
                .default_height(400)
                .resizable(true)
                .build();
            window.set_size_request(770, 400);
            let window_weak = window.downgrade();

            // Main vertical box
            let vbox = gui_widgets::create_main_container();

            // ISO selection (inline label, increased height)
            let (iso_hbox, iso_entry, iso_button, download_button) = gui_widgets::create_iso_selection_widget();
            vbox.append(&iso_hbox);

            // --- OS label (for detection) ---
            let os_label = gui_widgets::create_os_label();
            vbox.append(&os_label);

            // Separator
            let sep1 = gui_widgets::create_separator();
            vbox.append(&sep1);

            // USB device selection (inline label, increased height)
            let (device_hbox, device_combo, refresh_button) = gui_widgets::create_device_selection_widget();
            vbox.append(&device_hbox);

            // Separator
            let sep2 = gtk4::Separator::new(Orientation::Horizontal);
            sep2.set_halign(gtk4::Align::Center);
            sep2.set_hexpand(true);
            sep2.set_width_request((720.0 * 0.8) as i32);
            vbox.append(&sep2);

            // --- Windows form group (hidden by default) ---
            let (windows_group, cluster_combo, dd_checkbox, bypass_tpm_cb, bypass_secure_boot_cb, bypass_ram_cb) = gui_widgets::create_windows_advanced_options();
            vbox.append(&windows_group);

            // --- Linux form group (hidden by default) ---
            let (linux_group, persistence_checkbox, table_type_combo) = gui_widgets::create_linux_advanced_options();
            persistence_checkbox.set_active(false);
            vbox.append(&linux_group);

            // Write and Advanced options buttons (side by side, centered)
            let (button_hbox, write_button, advanced_button) = gui_widgets::create_button_container();
            vbox.append(&button_hbox);

            // Move OS label below the buttons
            vbox.append(&os_label);

            // Log area
            let (log_label, log_view, log_scroll) = gui_widgets::create_log_area();
            vbox.append(&log_label);
            vbox.append(&log_scroll);

            // Add a progress bar below the log area
            let progress_bar = gui_widgets::create_progress_bar();
            vbox.append(&progress_bar);

            // --- Advanced options logic with toggle (refactored, reusable reset) ---
            let adv_open = std::rc::Rc::new(std::cell::Cell::new(false));
            let advanced_button_ref = std::rc::Rc::new(advanced_button.clone());
            // Extract reusable reset/close logic for advanced options
            let reset_advanced_options = {
                let windows_group = windows_group.clone();
                let linux_group = linux_group.clone();
                let cluster_combo = cluster_combo.clone();
                let dd_checkbox = dd_checkbox.clone();
                let bypass_tpm_cb = bypass_tpm_cb.clone();
                let bypass_secure_boot_cb = bypass_secure_boot_cb.clone();
                let bypass_ram_cb = bypass_ram_cb.clone();
                let persistence_checkbox = persistence_checkbox.clone();
                let os_label = os_label.clone();
                let advanced_button_ref = advanced_button_ref.clone();
                let adv_open = adv_open.clone();
                move || {
                    windows_group.set_visible(false);
                    linux_group.set_visible(false);
                    cluster_combo.set_active(Some(3));
                    dd_checkbox.set_active(false);
                    bypass_tpm_cb.set_active(false);
                    bypass_secure_boot_cb.set_active(false);
                    bypass_ram_cb.set_active(false);
                    persistence_checkbox.set_active(false);
                    os_label.set_text("");
                    advanced_button_ref.set_label("Advanced options");
                    adv_open.set(false);
                }
            };

            // --- Advanced options button handler ---
            {
                let is_elevating = std::rc::Rc::new(std::cell::Cell::new(false));
                let adv_open = adv_open.clone();
                let advanced_button_ref = advanced_button_ref.clone();
                let iso_entry = iso_entry.clone();
                let os_label = os_label.clone();
                let windows_group = windows_group.clone();
                let linux_group = linux_group.clone();
                let bypass_tpm_cb = bypass_tpm_cb.clone();
                let bypass_secure_boot_cb = bypass_secure_boot_cb.clone();
                let bypass_ram_cb = bypass_ram_cb.clone();
                let reset_advanced_options = reset_advanced_options.clone();
                // Global elevation counter
                static ELEVATION_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                advanced_button.connect_clicked(move |_| {
                    println!("[DEBUG] [{}:{}] Advanced options button clicked. adv_open={}, is_elevating={}", file!(), line!(), adv_open.get(), is_elevating.get());
                    if adv_open.get() {
                        println!("[DEBUG] [{}:{}] Closing advanced options.", file!(), line!());
                        reset_advanced_options();
                        return;
                    }
                    if is_elevating.get() {
                        println!("[DEBUG] [{}:{}] Elevation already in progress, ignoring click.", file!(), line!());
                        return;
                    }
                    let iso_path = iso_entry.text();
                    if iso_path.is_empty() {
                        println!("[DEBUG] [{}:{}] No ISO selected, cannot detect OS.", file!(), line!());
                        os_label.set_text("Please select an ISO first.");
                        return;
                    }
                    println!("[DEBUG] [{}:{}] Attempting user-mount OS detection...", file!(), line!());
                    let detected = crate::utils::is_windows_iso(&iso_path);
                    match detected {
                        Some(true) => {
                            println!("[DEBUG] [{}:{}] Detected Windows ISO (user-mount)", file!(), line!());
                            os_label.set_text("Detected: Windows ISO (mounted)");
                            windows_group.set_visible(true);
                            linux_group.set_visible(false);
                            advanced_button_ref.set_label("Close advanced options");
                            adv_open.set(true);
                            bypass_tpm_cb.set_active(false);
                            bypass_secure_boot_cb.set_active(false);
                            bypass_ram_cb.set_active(false);
                        },
                        Some(false) => {
                            println!("[DEBUG] [{}:{}] Detected Linux ISO (user-mount)", file!(), line!());
                            os_label.set_text("Detected: Linux ISO (mounted)");
                            windows_group.set_visible(false);
                            linux_group.set_visible(true);
                            advanced_button_ref.set_label("Close advanced options");
                            adv_open.set(true);
                            bypass_tpm_cb.set_active(false);
                            bypass_secure_boot_cb.set_active(false);
                            bypass_ram_cb.set_active(false);
                        },
                        None => {
                            println!("[DEBUG] [{}:{}] User-mount detection failed, requesting elevation...", file!(), line!());
                            is_elevating.set(true);
                            let prev = ELEVATION_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                            println!("[DEBUG] [{}:{}] is_elevating set to true, calling is_windows_iso... (ELEVATION_COUNT={})", file!(), line!(), prev);
                            let result = crate::utils::is_windows_iso(&iso_path);
                            println!("[DEBUG] [{}:{}] is_windows_iso returned: {:?}", file!(), line!(), result);
                            match result {
                                Some(true) => {
                                    println!("[DEBUG] [{}:{}] Detected Windows ISO (root mount)", file!(), line!());
                                    os_label.set_text("Detected: Windows ISO (root mount)");
                                    windows_group.set_visible(true);
                                    linux_group.set_visible(false);
                                    advanced_button_ref.set_label("Close advanced options");
                                    adv_open.set(true);
                                    bypass_tpm_cb.set_active(false);
                                    bypass_secure_boot_cb.set_active(false);
                                    bypass_ram_cb.set_active(false);
                                },
                                Some(false) => {
                                    println!("[DEBUG] [{}:{}] Detected Linux ISO (root mount)", file!(), line!());
                                    os_label.set_text("Detected: Linux ISO (root mount)");
                                    windows_group.set_visible(false);
                                    linux_group.set_visible(true);
                                    advanced_button_ref.set_label("Close advanced options");
                                    adv_open.set(true);
                                    bypass_tpm_cb.set_active(false);
                                    bypass_secure_boot_cb.set_active(false);
                                    bypass_ram_cb.set_active(false);
                                },
                                None => {
                                    println!("[DEBUG] [{}:{}] Could not detect OS type even with root", file!(), line!());
                                    os_label.set_text("Could not detect OS type (even with root)");
                                    reset_advanced_options();
                                },
                            }
                            is_elevating.set(false);
                            println!("[DEBUG] [{}:{}] is_elevating set to false after elevation attempt", file!(), line!());
                        }
                    }
                });
            }

            // --- ISO selection event handler (reset form groups, no auto-detect, no double picker) ---
            {
                let iso_entry = iso_entry.clone();
                let window_weak_browse = window_weak.clone();
                let reset_advanced_options = reset_advanced_options.clone();
                iso_button.connect_clicked(move |_| {
                    if let Some(window) = window_weak_browse.upgrade() {
                        let reset_advanced_options = reset_advanced_options.clone();
                        gui_dialogs::show_iso_file_chooser_dialog_app(
                            &window,
                            &iso_entry,
                            &os_label,
                            move || reset_advanced_options(),
                        );
                    }
                });
            }

            // --- Download ISO button handler ---
            {
                let window_weak_download = window_weak.clone();
                download_button.connect_clicked(move |_| {
                    if let Some(window) = window_weak_download.upgrade() {
                        gui_dialogs::show_iso_downloader_dialog(Some(&window));
                    }
                });
            }

            // --- USB device refresh functionality ---
            {
                let device_combo = device_combo.clone();
                refresh_button.connect_clicked(move |_| {
                    println!("[DEBUG] Refreshing USB device list...");
                    device_combo.remove_all();

                    let devices = crate::utils::list_usb_devices();
                    let device_count = devices.len();
                    if devices.is_empty() {
                        device_combo.append_text("(No USB devices found)");
                        device_combo.set_active(Some(0));
                    } else {
                        for (path, description) in devices {
                            device_combo.append_text(&format!("{} - {}", path, description));
                        }
                        device_combo.set_active(Some(0));
                    }
                    println!("[DEBUG] Found {} USB devices", device_count);
                });
            }

            // --- Write button functionality ---
            {
                let write_button = write_button.clone();
                let iso_entry = iso_entry.clone();
                let device_combo = device_combo.clone();
                let windows_group = windows_group.clone();
                let linux_group = linux_group.clone();
                let cluster_combo = cluster_combo.clone();
                let persistence_checkbox = persistence_checkbox.clone();
                let log_view = log_view.clone();
                let progress_bar = progress_bar.clone();
                let window_for_dialog = window.clone();

                write_button.clone().connect_clicked(move |_| {
                    let iso_path = iso_entry.text().to_string();
                    if iso_path.is_empty() {
                        let buffer = log_view.buffer();
                        buffer.set_text("ERROR: No ISO file selected\n");
                        return;
                    }

                    let active_device = device_combo.active_text().unwrap_or_default();
                    if active_device.is_empty() || active_device.contains("(refresh to list devices)") || active_device.contains("(No USB devices found)") {
                        let buffer = log_view.buffer();
                        buffer.set_text("ERROR: No USB device selected or no devices found\n");
                        return;
                    }

                    // Extract device path (before " - " separator)
                    let device_path = active_device.split(" - ").next().unwrap_or("").trim();
                    let device_path = device_path.to_string(); // Create owned copy
                    if device_path.is_empty() {
                        let buffer = log_view.buffer();
                        buffer.set_text("ERROR: Could not parse device path\n");
                        return;
                    }

                    println!("[DEBUG] Starting USB write: ISO={}, Device={}", iso_path, device_path);

                    // Update UI for write operation
                    write_button.set_sensitive(false);

                    // Configure infinite progress bar
                    progress_bar.set_fraction(0.0);
                    progress_bar.set_show_text(true);
                    progress_bar.set_text(Some("Preparing to write..."));
                    progress_bar.set_pulse_step(0.1);
                    progress_bar.set_visible(true);

                    let buffer = log_view.buffer();
                    let mut log_text = format!("Starting write operation:\n");
                    log_text.push_str(&format!("  ISO: {}\n", iso_path));
                    log_text.push_str(&format!("  Device: {}\n", device_path));

                    let mut persistence_config: Option<PersistenceConfig> = None;

                    // Determine write mode and options
                    // Prefer explicit detection over UI visibility to avoid falling back to Linux when the Windows group is hidden.
                    let detected_windows = crate::utils::is_windows_iso(&iso_path).unwrap_or(false);
                    let is_windows_mode = if windows_group.is_visible() {
                        true
                    } else {
                        detected_windows
                    };

                    let use_dd_mode = if is_windows_mode {
                        dd_checkbox.is_active()
                    } else {
                        false
                    };

                    let bypass_tpm = if is_windows_mode { bypass_tpm_cb.is_active() } else { false };
                    let bypass_secure_boot = if is_windows_mode { bypass_secure_boot_cb.is_active() } else { false };
                    let bypass_ram = if is_windows_mode { bypass_ram_cb.is_active() } else { false };

                    if is_windows_mode {
                        let cluster_idx = cluster_combo.active().unwrap_or(3) as usize;
                        let cluster_sizes = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536];
                        let cluster_size = *cluster_sizes.get(cluster_idx).unwrap_or(&4096);
                        let mode_label = if use_dd_mode { "Windows (direct dd mode)" } else { "Windows" };
                        log_text.push_str(&format!("  Mode: {} (cluster size: {} bytes)\n", mode_label, cluster_size));
                        if bypass_tpm || bypass_secure_boot || bypass_ram {
                            log_text.push_str(&format!(
                                "  Bypass options: TPM={} SecureBoot={} RAM={}\n",
                                bypass_tpm, bypass_secure_boot, bypass_ram
                            ));
                        }
                    } else if detected_windows {
                        // Windows detected but advanced panel not open; use default cluster size.
                        log_text.push_str("  Mode: Windows (auto-detected, cluster size: 4096 bytes)\n");
                    } else if linux_group.is_visible() {
                        let persistence = persistence_checkbox.is_active();
                        if persistence {
                            let table_type = match table_type_combo.active().unwrap_or(0) {
                                1 => PartitionTableType::Mbr,
                                _ => PartitionTableType::Gpt,
                            };
                            let persistence_type = match linux_persistence::detect_persistence_type(&iso_path) {
                                Ok(kind) => kind,
                                Err(e) => {
                                    let msg = format!("ERROR: Could not detect persistence type: {}\n", e);
                                    buffer.set_text(&msg);
                                    write_button.set_sensitive(true);
                                    progress_bar.set_text(Some("Error"));
                                    return;
                                }
                            };

                            let recommended_size = match linux_persistence::get_recommended_persistence_size(&iso_path, &device_path) {
                                Ok(size) => size,
                                Err(e) => {
                                    let msg = format!("ERROR: Could not calculate persistence size: {}\n", e);
                                    buffer.set_text(&msg);
                                    write_button.set_sensitive(true);
                                    progress_bar.set_text(Some("Error"));
                                    return;
                                }
                            };

                            let config = PersistenceConfig {
                                enabled: true,
                                size_mb: recommended_size,
                                persistence_type,
                                label: "persistence".to_string(),
                                partition_table: table_type,
                            };

                            if let Err(e) = linux_persistence::validate_persistence_config(&config) {
                                let msg = format!("ERROR: Invalid persistence configuration: {}\n", e);
                                buffer.set_text(&msg);
                                write_button.set_sensitive(true);
                                progress_bar.set_text(Some("Error"));
                                return;
                            }

                            log_text.push_str(&format!(
                                "  Mode: Linux (persistence: enabled, type: {:?}, size: {} MB)\n",
                                config.persistence_type, config.size_mb
                            ));
                            log_text.push_str(&format!(
                                "  Partition table: {:?}\n",
                                config.partition_table
                            ));
                            persistence_config = Some(config);
                        } else {
                            log_text.push_str("  Mode: Linux (persistence: disabled)\n");
                        }
                    } else {
                        // Default to Linux mode if no advanced group is visible
                        log_text.push_str("  Mode: Linux (persistence: disabled)\n");
                    }

                    buffer.set_text(&log_text);

                    // Show confirmation dialog before starting
                    let dialog = gui_dialogs::show_usb_write_confirmation_dialog(
                        Some(&window_for_dialog),
                        &device_path
                    );

                    let progress_bar_clone = progress_bar.clone();
                    let write_button_clone = write_button.clone();
                    let log_view_clone = log_view.clone();
                    let iso_path_clone = iso_path.clone();
                    let device_path_clone = device_path.clone();
                    let persistence_config_clone = persistence_config.clone();
                    let is_windows_mode_clone = is_windows_mode;
                    let use_dd_mode_clone = use_dd_mode;
                    let bypass_tpm_clone = bypass_tpm;
                    let bypass_secure_boot_clone = bypass_secure_boot;
                    let bypass_ram_clone = bypass_ram;
                    let window_for_dialog_clone = window_for_dialog.clone();

                    dialog.connect_response(move |dialog, response| {
                        dialog.close();

                        if response != gtk4::ResponseType::Ok {
                            write_button_clone.set_sensitive(true);
                            progress_bar_clone.set_fraction(0.0);
                            progress_bar_clone.set_show_text(false);
                            return;
                        }

                        if is_windows_mode_clone && use_dd_mode_clone {
                            // Show dd warning; cancel if user declines.
                            if !gui_dialogs::show_dd_mode_warning_dialog(&window_for_dialog_clone) {
                                write_button_clone.set_sensitive(true);
                                progress_bar_clone.set_fraction(0.0);
                                progress_bar_clone.set_show_text(false);
                                return;
                            }
                        }

                        let buffer = log_view_clone.buffer();
                        let start = buffer.start_iter();
                        let end = buffer.end_iter();
                        let mut current_text = buffer.text(&start, &end, false).to_string();
                        current_text.push_str("\n=== Starting write operation ===\n");
                        buffer.set_text(&current_text);

                        // Configure progress bar
                        progress_bar_clone.set_fraction(0.0);
                        progress_bar_clone.set_show_text(true);
                        progress_bar_clone.set_text(Some("Starting..."));
                        progress_bar_clone.set_visible(true);

                        // Keep UI responsive: run heavy work on a background thread
                        let (sender, receiver) = glib::MainContext::channel(Priority::default());
                        let pulse_running = std::rc::Rc::new(std::cell::Cell::new(true));
                        let pulse_flag = pulse_running.clone();
                        let progress_bar_anim = progress_bar_clone.clone();
                        glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
                            if !pulse_flag.get() {
                                return glib::ControlFlow::Break;
                            }
                            progress_bar_anim.pulse();
                            glib::ControlFlow::Continue
                        });

                        // UI receiver to update progress/log without blocking
                        {
                            let buffer_ui = log_view_clone.buffer();
                            let log_view_ui = log_view_clone.clone();
                            let progress_ui = progress_bar_clone.clone();
                            let write_button_ui = write_button_clone.clone();
                            receiver.attach(None, move |msg| {
                                match msg {
                                    WorkerMessage::Log(line) => {
                                        let start = buffer_ui.start_iter();
                                        let end = buffer_ui.end_iter();
                                        let mut text = buffer_ui.text(&start, &end, false).to_string();
                                        text.push_str(&line);
                                        if !text.ends_with('\n') {
                                            text.push('\n');
                                        }
                                        buffer_ui.set_text(&text);
                                        let mut end_iter = buffer_ui.end_iter();
                                        log_view_ui.scroll_to_iter(&mut end_iter, 0.0, true, 0.0, 1.0);
                                    }
                                    WorkerMessage::Status(status) => {
                                        progress_ui.set_text(Some(&status));
                                    }
                                    WorkerMessage::Done(result) => {
                                        pulse_running.set(false);
                                        progress_ui.set_fraction(1.0);
                                        write_button_ui.set_sensitive(true);

                                        let start = buffer_ui.start_iter();
                                        let end = buffer_ui.end_iter();
                                        let mut text = buffer_ui.text(&start, &end, false).to_string();

                                        match result {
                                            Ok(()) => {
                                                text.push_str("\n✓ Write operation completed successfully!\n");
                                                progress_ui.set_text(Some("Complete!"));
                                                let completion_dialog = gui_dialogs::show_usb_completion_dialog();
                                                completion_dialog.connect_response(|dialog, _| dialog.close());
                                                completion_dialog.show();
                                            }
                                            Err(e) => {
                                                text.push_str(&format!("\n✗ Write operation failed: {}\n", e));
                                                progress_ui.set_text(Some("Failed"));
                                            }
                                        }

                                        buffer_ui.set_text(&text);
                                        let mut end_iter = buffer_ui.end_iter();
                                        log_view_ui.scroll_to_iter(&mut end_iter, 0.0, true, 0.0, 1.0);
                                    }
                                }
                                glib::ControlFlow::Continue
                            });
                        }

                        // Spawn worker thread
                        let iso_for_thread = iso_path_clone.clone();
                        let device_for_thread = device_path_clone.clone();
                        let persistence_for_thread = persistence_config_clone.clone();
                        let sender_clone = sender.clone();
                        std::thread::spawn(move || {
                            let send = |m| { let _ = sender_clone.send(m); };
                            if is_windows_mode_clone {
                                if use_dd_mode_clone {
                                    send(WorkerMessage::Log("Starting Windows direct dd write (not recommended)...".into()));
                                    send(WorkerMessage::Status("Writing image (dd)...".into()));
                                    let mut logger = ChannelWriter { sender: sender_clone.clone() };
                                    let result = crate::flows::windows_flow::write_windows_iso_direct_dd(
                                        &iso_for_thread,
                                        &device_for_thread,
                                        &mut logger
                                    ).map_err(|e| e.to_string());
                                    let _ = sender_clone.send(WorkerMessage::Done(result));
                                    return;
                                }

                                send(WorkerMessage::Log("Starting Windows dual-partition write...".into()));
                                if bypass_tpm_clone || bypass_secure_boot_clone || bypass_ram_clone {
                                    send(WorkerMessage::Log(format!(
                                        "Bypass options selected: TPM={} SecureBoot={} RAM={}",
                                        bypass_tpm_clone, bypass_secure_boot_clone, bypass_ram_clone
                                    )));
                                }
                                send(WorkerMessage::Status("Creating partitions...".into()));
                                let mut logger = ChannelWriter { sender: sender_clone.clone() };
                                let mut flags = crate::windows::unattend::UnattendFlags::empty();
                                if bypass_tpm_clone {
                                    flags |= crate::windows::unattend::UnattendFlags::BYPASS_TPM;
                                }
                                if bypass_secure_boot_clone {
                                    flags |= crate::windows::unattend::UnattendFlags::BYPASS_SECURE_BOOT;
                                }
                                if bypass_ram_clone {
                                    flags |= crate::windows::unattend::UnattendFlags::BYPASS_RAM;
                                }
                                let result = crate::flows::windows_flow::write_windows_iso_to_usb_with_bypass(
                                    &iso_for_thread,
                                    &device_for_thread,
                                    false,
                                    if flags.is_empty() { None } else { Some(flags) },
                                    &mut logger
                                ).map(|_| ()).map_err(|e| e.to_string());
                                let _ = sender_clone.send(WorkerMessage::Done(result));
                            } else {
                                send(WorkerMessage::Log("Starting Linux ISO write...".into()));
                                send(WorkerMessage::Log("Writing image using dd...".into()));
                                send(WorkerMessage::Status("Writing image...".into()));
                                let result = crate::flows::linux_flow::write_iso_to_usb_with_persistence(
                                    &iso_for_thread,
                                    &device_for_thread,
                                    &mut std::io::Cursor::new(Vec::new()),
                                    persistence_for_thread
                                ).map_err(|e| e.to_string());
                                send(WorkerMessage::Status("Finalizing persistence (if enabled)...".into()));
                                let _ = sender_clone.send(WorkerMessage::Done(result));
                            }
                        });
                    });

                    dialog.show();
                });
            }

            // Set the main vbox as the window content
            window.set_child(Some(&vbox));
            window.show();

            // Show Flatpak permission dialog if needed
            if needs_root && is_flatpak {
                gui_dialogs::show_flatpak_instructions_dialog(&window);
            }
        }
    });

    app.run();
}
