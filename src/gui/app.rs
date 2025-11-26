use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar, MessageDialog, ButtonsType, MessageType, ResponseType};
use glib::{self, Priority};
use std::io;

use crate::flows::linux_persistence::{self, PersistenceConfig};

enum WorkerMessage {
    Log(String),
    Status(String),
    Done(Result<(), String>),
}

/// Write Linux ISO (use original working version)
fn write_linux_iso_with_progress(iso_path: &str, usb_device: &str, _log_view: &TextView, _progress_bar: &ProgressBar) -> io::Result<()> {
    // Use the original, working implementation
    crate::flows::linux_flow::write_iso_to_usb(iso_path, usb_device, &mut std::io::Cursor::new(Vec::new()))
}

/// Write Windows ISO (use original working version)
fn write_windows_iso_with_progress(iso_path: &str, usb_device: &str, _log_view: &TextView, _progress_bar: &ProgressBar) -> io::Result<()> {
    // Use the original, working implementation
    crate::flows::windows_flow::write_windows_iso_to_usb(iso_path, usb_device, false, &mut std::io::Cursor::new(Vec::new()))
}

/// Format bytes in human readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn run_gui(needs_root: bool, is_flatpak: bool) {
    // Apply user's visual theme settings before creating GUI
    crate::utils::apply_user_theme();

    let app = Application::builder()
        .application_id("com.github.vicrodh.MajUSB")
        .build();

    app.connect_activate(move |app| {
        // Check for required packages before showing the main window
        if let Some((_, install_cmd)) = crate::utils::check_required_packages() {
            let dialog = gtk4::Dialog::with_buttons(
                Some("Missing Required Packages"),
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                &[("OK", gtk4::ResponseType::Ok)],
            );
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
            let vbox = GtkBox::new(Orientation::Vertical, 12);
            vbox.set_margin_top(16);
            vbox.set_margin_bottom(16);
            vbox.set_margin_start(16);
            vbox.set_margin_end(16);

            // ISO selection (inline label, increased height)
            let iso_hbox = GtkBox::new(Orientation::Horizontal, 8);
            let iso_label = Label::new(Some("ISO Image:"));
            iso_label.set_halign(gtk4::Align::Start);
            iso_label.set_valign(gtk4::Align::Center);
            iso_label.set_margin_top(3);
            iso_label.set_margin_bottom(3);
            let iso_entry = Entry::builder().placeholder_text("Select ISO file...").build();
            iso_entry.set_hexpand(true);
            iso_entry.set_margin_top(3);
            iso_entry.set_margin_bottom(3);
            let iso_button = Button::builder()
                .icon_name("document-open")
                .build();
            iso_button.set_hexpand(false);
            iso_button.set_halign(gtk4::Align::End);
            iso_button.set_tooltip_text(Some("Browse for ISO file"));
            iso_button.set_margin_top(3);
            iso_button.set_margin_bottom(3);
            iso_hbox.append(&iso_label);
            iso_hbox.append(&iso_entry);
            iso_hbox.append(&iso_button);
            iso_hbox.set_homogeneous(false);
            iso_hbox.set_spacing(8);
            iso_hbox.set_size_request(0, 0);
            iso_entry.set_width_chars(40);
            vbox.append(&iso_hbox);

            // --- OS label (for detection) ---
            let os_label = Label::new(None);
            vbox.append(&os_label);

            // Separator
            let sep1 = gtk4::Separator::new(Orientation::Horizontal);
            sep1.set_halign(gtk4::Align::Center);
            sep1.set_hexpand(true);
            sep1.set_width_request((720.0 * 0.8) as i32);
            vbox.append(&sep1);

            // USB device selection (inline label, increased height)
            let device_hbox = GtkBox::new(Orientation::Horizontal, 8);
            let device_label = Label::new(Some("USB Device:"));
            device_label.set_halign(gtk4::Align::Start);
            device_label.set_valign(gtk4::Align::Center);
            device_label.set_margin_top(3);
            device_label.set_margin_bottom(3);
            let device_combo = ComboBoxText::new();
            device_combo.set_hexpand(true);
            device_combo.set_margin_top(3);
            device_combo.set_margin_bottom(3);
            device_combo.append_text("(refresh to list devices)");
            let refresh_button = Button::builder()
                .icon_name("view-refresh")
                .build();
            refresh_button.set_hexpand(false);
            refresh_button.set_halign(gtk4::Align::End);
            refresh_button.set_tooltip_text(Some("Refresh device list"));
            refresh_button.set_margin_top(3);
            refresh_button.set_margin_bottom(3);
            device_hbox.append(&device_label);
            device_hbox.append(&device_combo);
            device_hbox.append(&refresh_button);
            device_hbox.set_homogeneous(false);
            device_hbox.set_spacing(8);
            vbox.append(&device_hbox);

            // Separator
            let sep2 = gtk4::Separator::new(Orientation::Horizontal);
            sep2.set_halign(gtk4::Align::Center);
            sep2.set_hexpand(true);
            sep2.set_width_request((720.0 * 0.8) as i32);
            vbox.append(&sep2);

            // --- Windows form group (hidden by default) ---
            let windows_group = GtkBox::new(Orientation::Vertical, 8);
            windows_group.set_visible(false);
            // Advanced options title as a horizontal bar
            let windows_title_bar = GtkBox::new(Orientation::Horizontal, 4);
            let left_sep = gtk4::Separator::new(Orientation::Horizontal);
            left_sep.set_hexpand(true);
            let adv_label = Label::new(Some("Advanced options"));
            adv_label.set_halign(gtk4::Align::Center);
            adv_label.set_markup("<b>Advanced options</b>");
            let right_sep = gtk4::Separator::new(Orientation::Horizontal);
            right_sep.set_hexpand(true);
            windows_title_bar.append(&left_sep);
            windows_title_bar.append(&adv_label);
            windows_title_bar.append(&right_sep);
            windows_group.append(&windows_title_bar);
            let cluster_label = Label::new(Some("Cluster Size:"));
            let cluster_sizes = vec![
                ("512 bytes", 512),
                ("1K", 1024),
                ("2K", 2048),
                ("4K", 4096),
                ("8K", 8192),
                ("16K", 16384),
                ("32K", 32768),
                ("64K", 65536),
            ];
            let cluster_combo = ComboBoxText::new();
            for (label, _val) in &cluster_sizes {
                cluster_combo.append_text(label);
            }
            cluster_combo.set_active(Some(3)); // Default to 4K (4096 bytes)
            windows_group.append(&cluster_label);
            windows_group.append(&cluster_combo);
            vbox.append(&windows_group);

            // --- Linux form group (hidden by default) ---
            let linux_group = GtkBox::new(Orientation::Vertical, 8);
            linux_group.set_visible(false);
            // Advanced options title as a horizontal bar (reuse for Linux)
            let linux_title_bar = GtkBox::new(Orientation::Horizontal, 4);
            let left_sep2 = gtk4::Separator::new(Orientation::Horizontal);
            left_sep2.set_hexpand(true);
            let adv_label2 = Label::new(Some("Advanced options"));
            adv_label2.set_halign(gtk4::Align::Center);
            adv_label2.set_markup("<b>Advanced options</b>");
            let right_sep2 = gtk4::Separator::new(Orientation::Horizontal);
            right_sep2.set_hexpand(true);
            linux_title_bar.append(&left_sep2);
            linux_title_bar.append(&adv_label2);
            linux_title_bar.append(&right_sep2);
            linux_group.append(&linux_title_bar);
            let persistence_checkbox = gtk4::CheckButton::with_label("Add persistence");
            persistence_checkbox.set_active(false);
            linux_group.append(&persistence_checkbox);
            vbox.append(&linux_group);

            // Write and Advanced options buttons (side by side, centered)
            let button_hbox = GtkBox::new(Orientation::Horizontal, 8);
            button_hbox.set_halign(gtk4::Align::Center);
            let write_button = Button::with_label("Write to USB");
            let advanced_button = Button::with_label("Advanced options");
            button_hbox.append(&write_button);
            button_hbox.append(&advanced_button);
            vbox.append(&button_hbox);

            // Move OS label below the buttons
            vbox.append(&os_label);

            // Log area
            let log_label = Label::new(Some("Log:"));
            let log_view = TextView::new();
            log_view.set_editable(false);
            log_view.set_wrap_mode(gtk4::WrapMode::Word);
            log_view.set_monospace(true);
            log_view.set_vexpand(true);
            log_view.set_hexpand(true);
            log_view.set_margin_top(4);
            log_view.set_margin_bottom(4);
            log_view.set_margin_start(4);
            log_view.set_margin_end(4);
            log_view.set_justification(gtk4::Justification::Left);
            log_view.set_cursor_visible(false);
            log_view.set_left_margin(10);
            log_view.set_right_margin(10);
            log_view.set_top_margin(4);
            log_view.set_bottom_margin(4);
            let log_scroll = ScrolledWindow::builder().min_content_height(100).child(&log_view).build();
            vbox.append(&log_label);
            vbox.append(&log_scroll);

            // Add a progress bar below the log area
            let progress_bar = ProgressBar::new();
            progress_bar.set_show_text(false);
            progress_bar.set_fraction(0.0);
            vbox.append(&progress_bar);

            // Clone os_label for use in write button
            let os_label_write = os_label.clone();

            // --- Advanced options logic with toggle (refactored, reusable reset) ---
            let adv_open = std::rc::Rc::new(std::cell::Cell::new(false));
            let advanced_button_ref = std::rc::Rc::new(advanced_button.clone());
            // Extract reusable reset/close logic for advanced options
            let reset_advanced_options = {
                let windows_group = windows_group.clone();
                let linux_group = linux_group.clone();
                let cluster_combo = cluster_combo.clone();
                let persistence_checkbox = persistence_checkbox.clone();
                let os_label = os_label.clone();
                let advanced_button_ref = advanced_button_ref.clone();
                let adv_open = adv_open.clone();
                move || {
                    windows_group.set_visible(false);
                    linux_group.set_visible(false);
                    cluster_combo.set_active(Some(3));
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
                let cluster_combo = cluster_combo.clone();
                let persistence_checkbox = persistence_checkbox.clone();
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
                        },
                        Some(false) => {
                            println!("[DEBUG] [{}:{}] Detected Linux ISO (user-mount)", file!(), line!());
                            os_label.set_text("Detected: Linux ISO (mounted)");
                            windows_group.set_visible(false);
                            linux_group.set_visible(true);
                            advanced_button_ref.set_label("Close advanced options");
                            adv_open.set(true);
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
                                },
                                Some(false) => {
                                    println!("[DEBUG] [{}:{}] Detected Linux ISO (root mount)", file!(), line!());
                                    os_label.set_text("Detected: Linux ISO (root mount)");
                                    windows_group.set_visible(false);
                                    linux_group.set_visible(true);
                                    advanced_button_ref.set_label("Close advanced options");
                                    adv_open.set(true);
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
                        dialog.set_current_folder(Some(&gfile));

                        let iso_entry_clone2 = iso_entry.clone();
                        let reset_advanced_options = reset_advanced_options.clone();
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
                    let is_windows_mode = windows_group.is_visible();

                    if is_windows_mode {
                        let cluster_idx = cluster_combo.active().unwrap_or(3) as usize;
                        let cluster_sizes = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536];
                        let cluster_size = *cluster_sizes.get(cluster_idx).unwrap_or(&4096);
                        log_text.push_str(&format!("  Mode: Windows (cluster size: {} bytes)\n", cluster_size));
                    } else if linux_group.is_visible() {
                        let persistence = persistence_checkbox.is_active();
                        if persistence {
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
                    let dialog = gtk4::MessageDialog::builder()
                        .text("Confirm USB Write Operation")
                        .secondary_text(&format!("This will completely erase:\n{}\n\nProceed with write operation?", device_path))
                        .buttons(gtk4::ButtonsType::OkCancel)
                        .message_type(gtk4::MessageType::Warning)
                        .build();

                    let progress_bar_clone = progress_bar.clone();
                    let write_button_clone = write_button.clone();
                    let log_view_clone = log_view.clone();
                    let iso_path_clone = iso_path.clone();
                    let device_path_clone = device_path.clone();
                    let persistence_config_clone = persistence_config.clone();
                    let is_windows_mode_clone = is_windows_mode;

                    dialog.connect_response(move |dialog, response| {
                        dialog.close();

                        if response != gtk4::ResponseType::Ok {
                            write_button_clone.set_sensitive(true);
                            progress_bar_clone.set_fraction(0.0);
                            progress_bar_clone.set_show_text(false);
                            return;
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
                                                let completion_dialog = gtk4::MessageDialog::builder()
                                                    .text("USB creation complete!")
                                                    .message_type(gtk4::MessageType::Info)
                                                    .buttons(gtk4::ButtonsType::Ok)
                                                    .build();
                                                completion_dialog.connect_response(|dialog, _| dialog.close());
                                                completion_dialog.show();
                                            }
                                            Err(e) => {
                                                text.push_str(&format!("\n✗ Write operation failed: {}\n", e));
                                                progress_ui.set_text(Some("Failed"));
                                            }
                                        }

                                        buffer_ui.set_text(&text);
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
                                send(WorkerMessage::Log("Starting Windows dual-partition write...".into()));
                                send(WorkerMessage::Status("Creating partitions...".into()));
                                let result = crate::flows::windows_flow::write_windows_iso_to_usb(
                                    &iso_for_thread,
                                    &device_for_thread,
                                    false,
                                    &mut std::io::Cursor::new(Vec::new())
                                ).map_err(|e| e.to_string());
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
                show_flatpak_instructions_dialog(&window);
            }
        }
    });

    app.run();
}

fn show_flatpak_instructions_dialog(window: &ApplicationWindow) {
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

    dialog.set_default_response(ResponseType::Ok);
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show();
}
