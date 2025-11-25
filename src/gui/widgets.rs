// Widget creation functions (ISO selection, device selection, etc.)

use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Button, ComboBoxText, Entry, FileChooserAction, FileChooserDialog, FileFilter, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar};

/// Create ISO selection widget (label + entry + browse button)
pub fn create_iso_selection_widget() -> (GtkBox, Entry, Button) {
    let iso_hbox = GtkBox::new(Orientation::Horizontal, 8);
    let iso_label = Label::new(Some("ISO Image:"));
    iso_label.set_halign(gtk4::Align::Start);
    iso_label.set_valign(gtk4::Align::Center);
    iso_label.set_margin_top(3);
    iso_label.set_margin_bottom(3);
    let iso_entry = Entry::builder()
        .placeholder_text("Select ISO file...")
        .build();
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

    (iso_hbox, iso_entry, iso_button)
}

/// Create device selection widget (label + combo + refresh button)
pub fn create_device_selection_widget() -> (GtkBox, ComboBoxText, Button) {
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

    (device_hbox, device_combo, refresh_button)
}

/// Create separator widget
pub fn create_separator() -> gtk4::Separator {
    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_halign(gtk4::Align::Center);
    sep.set_hexpand(true);
    sep.set_width_request((720.0 * 0.8) as i32);
    sep
}

/// Create OS detection label
pub fn create_os_label() -> Label {
    Label::new(None)
}

/// Create advanced options section (cluster size for Windows)
pub fn create_windows_advanced_options() -> GtkBox {
    let windows_group = GtkBox::new(Orientation::Vertical, 6);
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
    cluster_combo.set_active(Some(3)); // Default to 4K

    windows_group.append(&Label::new(Some("Cluster Size:")));
    windows_group.append(&cluster_combo);

    // Store cluster sizes as attached data for later use
    let cluster_sizes_vec: Vec<(String, u64)> = cluster_sizes.iter()
        .map(|(label, val)| (label.to_string(), *val))
        .collect();
    unsafe { cluster_combo.set_data::<Vec<(String, u64)>>("cluster_sizes", cluster_sizes_vec); }

    windows_group
}

/// Create advanced options section (persistence for Linux)
pub fn create_linux_advanced_options() -> GtkBox {
    let linux_group = GtkBox::new(Orientation::Vertical, 6);
    let persistence_checkbox = gtk4::CheckButton::builder()
        .label("Enable persistence (store changes)")
        .build();
    linux_group.append(&Label::new(Some("Linux Options:")));
    linux_group.append(&persistence_checkbox);

    linux_group
}

/// Create write button
pub fn create_write_button() -> Button {
    Button::with_label("Write to USB")
}

/// Create log area with scrolled window
pub fn create_log_area() -> (Label, TextView, ScrolledWindow) {
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

    (log_label, log_view, log_scroll)
}

/// Create progress bar
pub fn create_progress_bar() -> ProgressBar {
    let progress_bar = ProgressBar::new();
    progress_bar.set_show_text(false);
    progress_bar.set_fraction(0.0);
    progress_bar
}

/// Create advanced options toggle button
pub fn create_advanced_button() -> Button {
    Button::with_label("Advanced Options ▼")
}
