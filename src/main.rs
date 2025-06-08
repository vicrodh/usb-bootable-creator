mod flows;
mod gui;
mod utils;

use gui::run_gui;

fn main() {
    // Request privilege escalation at startup
    utils::ensure_root();
    run_gui();
}
