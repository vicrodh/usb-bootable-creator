use rust_usb_bootable_creator::utils;
use rust_usb_bootable_creator::flows::windows_flow;
use rust_usb_bootable_creator::flows::linux_flow;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cli_helper <iso_path> <usb_device>");
        std::process::exit(1);
    }
    let iso_path = &args[1];
    let usb_device = &args[2];
    // Optionally: parse use_wim and cluster from args

    // Detect OS type (now as root)
    let is_win = utils::is_windows_iso(iso_path)
        .unwrap_or_else(|| {
            eprintln!("Detection failed, assuming Linux ISO");
            false
        });
    if is_win {
        println!("Detected: Windows ISO");
        let mut log_buf = Vec::new();
        let result = windows_flow::write_windows_iso_to_usb(
            iso_path, usb_device, false, &mut log_buf
        );
        print!("{}", String::from_utf8_lossy(&log_buf));
        if let Err(e) = result {
            eprintln!("Failed to write ISO: {}", e);
            std::process::exit(1);
        }
    } else {
        println!("Detected: Linux ISO");
        let mut log_buf = Vec::new();
        let result = linux_flow::write_iso_to_usb(
            iso_path, usb_device, &mut log_buf
        );
        print!("{}", String::from_utf8_lossy(&log_buf));
        if let Err(e) = result {
            eprintln!("Failed to write ISO: {}", e);
            std::process::exit(1);
        }
    }
    println!("Done!");
}
