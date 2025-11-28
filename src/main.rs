mod flows;
mod gui;
mod utils;
mod error;
mod windows;

use gui::run_gui;

fn main() {
    // Check environment and handle permissions appropriately
    let (needs_root, is_flatpak_env) = utils::check_root_requirements();

    if is_flatpak_env {
        // In Flatpak, run GUI and show instructions dialog
        run_gui(needs_root, true);
    } else if needs_root {
        // Normal execution: request root before GUI
        utils::ensure_root_normal();
        run_gui(false, false);
    } else {
        // Already root, run GUI
        run_gui(false, false);
    }
}
