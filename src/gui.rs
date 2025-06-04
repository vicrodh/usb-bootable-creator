use gtk4::glib::{self, clone, ControlFlow};
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar};

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
        let window_weak = window.downgrade();

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

        // Write button (call cli_helper via pkexec for detection + write, with real-time feedback)
        let log_view_clone = log_view.clone();
        let iso_entry_clone = iso_entry.clone();
        let device_combo_clone = device_combo.clone();
        let os_label_clone = os_label.clone();
        let progress_bar_clone = progress_bar.clone();
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
            progress_bar_clone.set_fraction(0.0);
            progress_bar_clone.set_show_text(false);
            progress_bar_clone.set_pulse_step(0.1);
            progress_bar_clone.set_visible(true);

            // --- Animate the progress bar in the main thread ---
            let (pulse_stop_tx, pulse_stop_rx) = std::sync::mpsc::channel();
            {
                let progress_bar_anim = progress_bar_clone.clone();
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
            let iso_path = iso_path.clone();
            let usb_device = usb_device.to_string();
            let pulse_stop_tx2 = pulse_stop_tx.clone();
            std::thread::spawn(move || {
                use std::process::{Command, Stdio};
                use std::io::{BufRead, BufReader};
                let cli_helper_path = "./target/debug/cli_helper";
                let mut child = match Command::new("pkexec")
                    .arg(cli_helper_path)
                    .arg(&iso_path)
                    .arg(&usb_device)
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
            let os_label_clone2 = os_label_clone.clone();
            let progress_bar_clone2 = progress_bar_clone.clone();
            let log_view_clone2 = log_view_clone.clone();
            let window_weak2 = window_weak.clone();
            gtk4::glib::idle_add_local(move || {
                while let Ok(msg) = receiver.try_recv() {
                    match msg {
                        Ok(line) => {
                            if line == "__PROCESS_DONE__" {
                                progress_bar_clone2.set_visible(false);
                                // Show completion dialog
                                if let Some(window) = window_weak2.upgrade() {
                                    let dialog = gtk4::MessageDialog::builder()
                                        .transient_for(&window)
                                        .modal(true)
                                        .text("USB creation complete!")
                                        .message_type(gtk4::MessageType::Info)
                                        .buttons(gtk4::ButtonsType::Ok)
                                        .build();
                                    dialog.connect_response(|dialog, _| dialog.close());
                                    dialog.show();
                                }
                                // Auto-scroll to end
                                let buffer = log_view_clone2.buffer();
                                let mut end_iter = buffer.end_iter();
                                log_view_clone2.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 1.0);
                                return glib::ControlFlow::Break;
                            }
                            // Insert the message
                            let mut end_iter = buffer_clone.end_iter();
                            buffer_clone.insert(&mut end_iter, &format!("{}\n", line));
                            // Show copy message after partitioning
                            if line.contains("Formatting INSTALL as NTFS") || line.contains("Formatting INSTALL as FAT32") {
                                let mut end_iter = buffer_clone.end_iter();
                                buffer_clone.insert(&mut end_iter, "I'm copying the files to the new device, please wait\n");
                            }
                            // Auto-scroll to end
                            let buffer = log_view_clone2.buffer();
                            let mut end_iter = buffer.end_iter();
                            log_view_clone2.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 1.0);
                            if line.contains("Windows ISO") {
                                os_label_clone2.set_text("Detected: Windows ISO");
                            } else if line.contains("Linux ISO") {
                                os_label_clone2.set_text("Detected: Linux ISO");
                            }
                        }
                        Err(err) => {
                            buffer_clone.set_text(&err);
                            progress_bar_clone2.set_visible(false);
                            // Auto-scroll to end
                            let buffer = log_view_clone2.buffer();
                            let mut end_iter = buffer.end_iter();
                            log_view_clone2.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 1.0);
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
        });

        window.set_child(Some(&vbox));
        window.present();
    });

    app.run();
}
