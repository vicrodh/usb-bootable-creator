use gtk4::glib::{self, clone, ControlFlow};
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar};

pub fn run_gui() {
    let app = Application::builder()
        .application_id("com.example.usbbootablecreator")
        .build();

    app.connect_activate(|app| {
        // Check for required packages before showing the main window
        if let Some((_, install_cmd)) = crate::utils::check_required_packages() {
            // Show a dialog with a text area and copy button
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
            // Main window
            let window = ApplicationWindow::builder()
                .application(app)
                .title("MajUSB Bootable Creator")
                .default_width(830) // Increased width by 130px total (80px before + 50px now)
                .default_height(400)
                .resizable(true)
                .build();
            window.set_size_request(770, 400); // Cap max width at 770px
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

            // Separator
            let sep1 = gtk4::Separator::new(Orientation::Horizontal);
            sep1.set_halign(gtk4::Align::Center);
            sep1.set_hexpand(true);
            sep1.set_width_request((720.0 * 0.8) as i32);
            vbox.append(&sep1);

            // Label to show detected OS type
            let os_label = Label::new(None);
            vbox.append(&os_label);

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

            // Cluster size (valid NTFS values only)
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
            vbox.append(&Label::new(Some("Cluster Size:")));
            vbox.append(&cluster_combo);

            // Write button
            let write_button = Button::with_label("Write to USB");
            vbox.append(&write_button);

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

            // --- Event handlers ---
            // ISO browse (no detection here)
            let iso_entry_clone = iso_entry.clone();
            let window_weak_browse = window_weak.clone();
            iso_button.connect_clicked(move |_| {
                if let Some(window) = window_weak_browse.upgrade() {
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
            use std::rc::Rc;
            let refresh_devices = Rc::new({
                let device_combo_clone = device_combo_clone.clone();
                move || {
                    device_combo_clone.remove_all();
                    let devices = crate::utils::list_usb_devices();
                    if devices.is_empty() {
                        device_combo_clone.append_text("No USB devices found");
                    } else {
                        for (dev, label) in devices {
                            device_combo_clone.append_text(&format!("{} ({})", dev, label));
                        }
                    }
                }
            });
            let refresh_devices_btn = refresh_devices.clone();
            refresh_button.connect_clicked(move |_| {
                (refresh_devices_btn)();
            });
            // Initial refresh as soon as possible (run after window is shown)
            let refresh_devices_start = refresh_devices.clone();
            window.connect_map(move |_| {
                (refresh_devices_start)();
            });

            // Write button (call cli_helper via pkexec for detection + write, with real-time feedback)
            let log_view_clone = log_view.clone();
            let iso_entry_clone = iso_entry.clone();
            let device_combo_clone = device_combo.clone();
            let os_label_clone = os_label.clone();
            let progress_bar_clone = progress_bar.clone();
            let cluster_combo_clone = cluster_combo.clone();
            let app_clone = app.clone(); // Pass app into closure
            write_button.connect_clicked(move |_| {
                let buffer = log_view_clone.buffer();
                let iso_path = iso_entry_clone.text();
                // Instead of borrowing device_text, clone it to a String for all uses below
                let device_text = device_combo_clone.active_text().map(|s| s.to_string()).unwrap_or_default();
                let usb_device = device_text.split_whitespace().next().unwrap_or("").to_string();
                if iso_path.is_empty() || usb_device.is_empty() {
                    buffer.set_text("Please select an ISO and a USB device.");
                    return;
                }
                // Confirmation dialog before proceeding
                let parent_window = window_weak.upgrade();
                let device_name = if device_text.is_empty() { "(unknown)".to_string() } else { device_text.clone() };
                let confirm_dialog = gtk4::Dialog::with_buttons(
                    Some("Confirm USB Overwrite"),
                    parent_window.as_ref(),
                    gtk4::DialogFlags::MODAL,
                    &[ ("No", gtk4::ResponseType::No), ("Yes", gtk4::ResponseType::Yes) ],
                );
                let content = confirm_dialog.content_area();
                // --- Simple confirmation dialog (known-good, no vbox, no margin, no markup, no button margin logic) ---
                let label1 = Label::new(Some(&format!(
                    "All data on {} will be erased!",
                    device_name
                )));
                let label2 = Label::new(Some("Are you sure you want to continue?"));
                content.append(&label1);
                content.append(&label2);
                confirm_dialog.set_default_response(gtk4::ResponseType::Yes);
                // Clone all GTK objects that are used in inner closures to avoid move errors
                let log_view_clone2 = log_view_clone.clone();
                let progress_bar_clone2 = progress_bar_clone.clone();
                let cluster_combo_clone2 = cluster_combo_clone.clone();
                let os_label_clone2 = os_label_clone.clone();
                let app_clone2 = app_clone.clone();
                let window_weak2 = window_weak.clone();
                let cluster_sizes2 = cluster_sizes.clone();
                confirm_dialog.connect_response(move |dialog: &gtk4::Dialog, resp| {
                    if resp == gtk4::ResponseType::Yes {
                        dialog.close();
                        // --- Continue with the original write flow ---
                        buffer.set_text("Requesting elevated permissions and writing to USB...\n");
                        progress_bar_clone2.set_fraction(0.0);
                        progress_bar_clone2.set_show_text(false);
                        progress_bar_clone2.set_pulse_step(0.1);
                        progress_bar_clone2.set_visible(true);

                        // --- Animate the progress bar in the main thread ---
                        let (pulse_stop_tx, pulse_stop_rx) = std::sync::mpsc::channel();
                        {
                            let progress_bar_anim = progress_bar_clone2.clone();
                            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), clone!(@weak progress_bar_anim => @default-return ControlFlow::Break, move || {
                                if pulse_stop_rx.try_recv().is_ok() {
                                    return ControlFlow::Break;
                                }
                                progress_bar_anim.pulse();
                                ControlFlow::Continue
                            }));
                        }

                        // --- Use a channel to communicate between thread and main ---
                        let (sender, receiver) = std::sync::mpsc::channel::<Result<String, String>>();
                        let iso_path2 = iso_path.clone();
                        let usb_device2 = usb_device.clone();
                        let cluster_index2 = cluster_combo_clone2.active().unwrap_or(3) as usize;
                        let cluster_bytes2 = cluster_sizes2[cluster_index2].1;
                        let cluster_arg2 = cluster_bytes2.to_string();
                        let pulse_stop_tx2 = pulse_stop_tx.clone();
                        std::thread::spawn(move || {
                            use std::process::{Command, Stdio};
                            use std::io::{BufRead, BufReader};
                            let cli_helper_path = "./target/debug/cli_helper";
                            let mut child = match Command::new("pkexec")
                                .arg(cli_helper_path)
                                .arg(&iso_path2)
                                .arg(&usb_device2)
                                .arg(&cluster_arg2)
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .spawn() {
                                    Ok(child) => child,
                                    Err(e) => {
                                        let _ = sender.send(Err(format!("Failed to launch helper: {}", e)));
                                        let _ = pulse_stop_tx2.send(());
                                        return;
                                    }
                                };
                            let stdout = child.stdout.take().unwrap();
                            let stderr = child.stderr.take().unwrap();
                            let reader = BufReader::new(stdout);
                            let err_reader = BufReader::new(stderr);
                            // Read stdout in real time
                            for line in reader.lines() {
                                let line = line.unwrap_or_default();
                                let _ = sender.send(Ok(line));
                            }
                            // Read stderr in real time
                            for line in err_reader.lines() {
                                let line = line.unwrap_or_default();
                                let _ = sender.send(Ok(line));
                            }
                            let _ = child.wait();
                            let _ = sender.send(Ok("__PROCESS_DONE__".to_string()));
                            let _ = pulse_stop_tx2.send(()); // Stop the progress bar pulsing
                        });

                        // --- Main thread: update GTK widgets safely ---
                        let buffer_clone = buffer.clone();
                        let os_label_clone2 = os_label_clone2.clone();
                        let progress_bar_clone2 = progress_bar_clone2.clone();
                        let log_view_clone2 = log_view_clone2.clone();
                        let window_weak2 = window_weak2.clone();
                        let app_clone2 = app_clone2.clone();
                        gtk4::glib::idle_add_local(move || {
                            while let Ok(msg) = receiver.try_recv() {
                                match msg {
                                    Ok(line) => {
                                        if line == "__PROCESS_DONE__" {
                                            progress_bar_clone2.set_visible(false);
                                            // Show completion dialog
                                            if let Some(window) = window_weak2.upgrade() {
                                                let dialog = gtk4::Dialog::with_buttons(
                                                    Some("USB creation complete!"),
                                                    Some(&window),
                                                    gtk4::DialogFlags::MODAL,
                                                    &[ ("OK", gtk4::ResponseType::Ok) ],
                                                );
                                                let content = dialog.content_area();
                                                // Center content with extra vertical/horizontal space
                                                let vbox = GtkBox::new(Orientation::Vertical, 0);
                                                vbox.set_valign(gtk4::Align::Center);
                                                vbox.set_halign(gtk4::Align::Center);
                                                vbox.set_margin_top(30);
                                                vbox.set_margin_bottom(30);
                                                vbox.set_margin_start(30);
                                                vbox.set_margin_end(30);
                                                let label = Label::new(Some("USB creation complete!"));
                                                label.set_margin_bottom(24); // More space below text
                                                label.set_justify(gtk4::Justification::Center);
                                                label.set_halign(gtk4::Align::Center);
                                                vbox.append(&label);
                                                // Find the OK button and set its width to 80% of dialog
                                                if let Some(action_area) = dialog.widget_for_response(gtk4::ResponseType::Ok) {
                                                    action_area.set_halign(gtk4::Align::Center);
                                                    action_area.set_valign(gtk4::Align::Center);
                                                    action_area.set_margin_top(10);
                                                    action_area.set_margin_bottom(10);
                                                    action_area.set_margin_start(10);
                                                    action_area.set_margin_end(10);
                                                    action_area.set_width_request((640.0 * 0.8) as i32);
                                                }
                                                vbox.set_spacing(0);
                                                content.set_halign(gtk4::Align::Center);
                                                content.set_valign(gtk4::Align::Center);
                                                content.set_margin_top(0);
                                                content.set_margin_bottom(0);
                                                content.set_margin_start(0);
                                                content.set_margin_end(0);
                                                content.append(&vbox);
                                                dialog.set_modal(true);
                                                dialog.set_default_response(gtk4::ResponseType::Ok);
                                                dialog.connect_response(|dialog, _| dialog.close());
                                                dialog.show();
                                            }
                                            // System notification on finish
                                            let notif = gio::Notification::new("MajUSB Bootable Creator");
                                            notif.set_body(Some("USB creation complete!"));
                                            app_clone2.send_notification(None, &notif);
                                            // Auto-scroll to end
                                            let buffer = log_view_clone2.buffer();
                                            let mut end_iter = buffer.end_iter();
                                            log_view_clone2.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 1.0);
                                            return glib::ControlFlow::Break;
                                        }
                                        // --- Parse and format step lines ---
                                        let mut markup = String::new();
                                        if let Some(caps) = line.strip_prefix("[STEP] ") {
                                            // Always show [STEP] lines, even if repeated, to allow real-time updates
                                            if let Some((counter, rest)) = caps.split_once(": ") {
                                                markup = format!(
                                                    "<span foreground='green' weight='bold'>[{}]</span> <span foreground='white' weight='bold'>{}</span>",
                                                    counter, rest
                                                );
                                            } else {
                                                markup = format!("<span foreground='white'>{}</span>", line);
                                            }
                                        } else if let Some(caps) = line.strip_prefix("[ERROR] ") {
                                            if let Some((counter, rest)) = caps.split_once(": ") {
                                                markup = format!(
                                                    "<span foreground='red' weight='bold'>[{}]</span> <span foreground='red' weight='bold'>{}</span>",
                                                    counter, rest
                                                );
                                            } else {
                                                markup = format!("<span foreground='red' weight='bold'>{}</span>", line);
                                            }
                                        } else {
                                            markup = format!("<span foreground='white'>{}</span>", line);
                                        }
                                        let buffer = log_view_clone2.buffer();
                                        let mut end_iter = buffer.end_iter();
                                        buffer.insert_markup(&mut end_iter, &format!("{}\n", markup));
                                        // Auto-scroll to end (force scroll after inserting)
                                        let buffer = log_view_clone2.buffer();
                                        let mut end_iter = buffer.end_iter();
                                        log_view_clone2.scroll_to_iter(&mut end_iter, 0.0, true, 0.0, 1.0);
                                        if line.contains("Windows ISO") {
                                            os_label_clone2.set_text("Detected: Windows ISO");
                                        } else if line.contains("Linux ISO") {
                                            os_label_clone2.set_text("Detected: Linux ISO");
                                        }
                                    }
                                    Err(err) => {
                                        let buffer = log_view_clone2.buffer();
                                        let mut end_iter = buffer.end_iter();
                                        buffer.insert_markup(&mut end_iter, &format!("<span foreground='red' weight='bold'>{}</span>\n", err));
                                        progress_bar_clone2.set_visible(false);
                                        // Auto-scroll to end
                                        let mut end_iter = buffer.end_iter();
                                        log_view_clone2.scroll_to_iter(&mut end_iter, 0.0, true, 0.0, 1.0);
                                        return glib::ControlFlow::Break;
                                    }
                                }
                            }
                            glib::ControlFlow::Continue
                        });
                    } else {
                        dialog.close();
                    }
                });
                confirm_dialog.show();
            });

            window.set_child(Some(&vbox));
            window.show();
        }
    });

    app.run();
}
