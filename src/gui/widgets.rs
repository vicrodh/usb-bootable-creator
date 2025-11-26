// Widget creation functions (ISO selection, device selection, etc.)

use gtk4::prelude::*;
use gtk4::{Button, ComboBoxText, Entry, Orientation, Box as GtkBox, Label, ScrolledWindow, TextView, ProgressBar, CheckButton};

/// Create main vertical box for the application
pub fn create_main_container() -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_bottom(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);
    vbox
}

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

    (iso_hbox, iso_entry, iso_button)
}

/// Create OS detection label
pub fn create_os_label() -> Label {
    Label::new(None)
}

/// Create separator widget
pub fn create_separator() -> gtk4::Separator {
    let sep = gtk4::Separator::new(Orientation::Horizontal);
    sep.set_halign(gtk4::Align::Center);
    sep.set_hexpand(true);
    sep.set_width_request((720.0 * 0.8) as i32);
    sep
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

    (device_hbox, device_combo, refresh_button)
}

/// Create Windows advanced options with title bar and cluster size selection
pub fn create_windows_advanced_options() -> (GtkBox, ComboBoxText) {
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

    (windows_group, cluster_combo)
}

/// Create Linux advanced options with title bar, persistence checkbox, and partition table type
pub fn create_linux_advanced_options() -> (GtkBox, CheckButton, ComboBoxText) {
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

    let persistence_checkbox = CheckButton::builder()
        .label("Enable persistence (store changes)")
        .build();
    linux_group.append(&persistence_checkbox);

    // Partition table type selector
    let table_type_combo = ComboBoxText::new();
    table_type_combo.append_text("GPT (default)");
    table_type_combo.append_text("MBR (msdos)");
    table_type_combo.set_active(Some(0));
    let table_type_label = Label::new(Some("Partition table type (persistence):"));
    linux_group.append(&table_type_label);
    linux_group.append(&table_type_combo);

    (linux_group, persistence_checkbox, table_type_combo)
}

/// Create button container with write and advanced buttons
pub fn create_button_container() -> (GtkBox, Button, Button) {
    let button_hbox = GtkBox::new(Orientation::Horizontal, 8);
    button_hbox.set_halign(gtk4::Align::Center);
    let write_button = Button::with_label("Write to USB");
    let advanced_button = Button::with_label("Advanced options");
    button_hbox.append(&write_button);
    button_hbox.append(&advanced_button);

    (button_hbox, write_button, advanced_button)
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
    log_view.set_top_margin(4);
    log_view.set_bottom_margin(4);
    log_view.set_left_margin(10);
    log_view.set_right_margin(10);
    log_view.set_justification(gtk4::Justification::Left);
    log_view.set_cursor_visible(false);
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