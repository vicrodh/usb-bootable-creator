use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar};

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
                        let iso_entry_clone2 = iso_entry.clone();
                        let reset_advanced_options = reset_advanced_options.clone();
                        dialog.connect_response(move |dialog, resp| {
                            if resp == gtk4::ResponseType::Ok {
                                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                                    let path_str = file.to_string_lossy().to_string();
                                    iso_entry_clone2.set_text(&path_str);
                                    // Call the reusable reset logic
                                    reset_advanced_options();
                                }
                            }
                            dialog.close();
                        });
                        dialog.show();
                    }
                });
            }

            // Set the main vbox as the window content
            window.set_child(Some(&vbox));
            window.show();
        }
    });

    app.run();
}
