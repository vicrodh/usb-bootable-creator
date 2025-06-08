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

            // --- ISO selection event handler ---
            let iso_entry_clone = iso_entry.clone();
            let window_weak_browse = window_weak.clone();
            iso_button.connect_clicked(move |_| {
                if let Some(window) = window_weak_browse.upgrade() {
                    let dialog = FileChooserDialog::new(
                        Some("Select ISO Image"),
                        Some(&window),
                        FileChooserAction::Open,
                        &[ ] // No buttons initially
                    );
                    dialog.add_button("Open", gtk4::ResponseType::Ok);
                    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
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

            // Set the main vbox as the window content
            window.set_child(Some(&vbox));
            window.show();
        }
    });

    app.run();
}
