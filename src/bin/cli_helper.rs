use rust_usb_bootable_creator::utils;
use rust_usb_bootable_creator::flows::windows_flow;
use rust_usb_bootable_creator::flows::linux_flow;

use std::env;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cli_helper <iso_path> <usb_device> [--use-dd-mode]");
        std::process::exit(1);
    }
    let iso_path = &args[1];
    let usb_device = &args[2];
    let use_dd_mode = args.iter().any(|a| a == "--use-dd-mode");
    let bypass_tpm = args.iter().any(|a| a == "--bypass-tpm");
    let bypass_secure_boot = args.iter().any(|a| a == "--bypass-secure-boot");
    let bypass_ram = args.iter().any(|a| a == "--bypass-ram");
    // Optionally: parse use_wim and cluster from args

    // Detect OS type (now as root)
    let is_win = utils::is_windows_iso(iso_path)
        .unwrap_or_else(|| {
            eprintln!("Detection failed, assuming Linux ISO");
            false
        });
    if is_win {
        println!("Detected: Windows ISO");
        io::stdout().flush().ok();
        if use_dd_mode {
            let result = windows_flow::write_windows_iso_direct_dd(
                iso_path, usb_device, &mut std::io::stdout()
            );
            if let Err(e) = result {
                eprintln!("Failed to write ISO (dd mode): {}", e);
                std::process::exit(1);
            }
        } else {
            let cluster_bytes: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4096);
            let mut flags = rust_usb_bootable_creator::windows::unattend::UnattendFlags::empty();
            if bypass_tpm {
                flags |= rust_usb_bootable_creator::windows::unattend::UnattendFlags::BYPASS_TPM;
            }
            if bypass_secure_boot {
                flags |= rust_usb_bootable_creator::windows::unattend::UnattendFlags::BYPASS_SECURE_BOOT;
            }
            if bypass_ram {
                flags |= rust_usb_bootable_creator::windows::unattend::UnattendFlags::BYPASS_RAM;
            }

            let result = windows_flow::write_windows_iso_to_usb_stream_with_bypass(
                iso_path, usb_device, cluster_bytes, if flags.is_empty() { None } else { Some(flags) }
            );
            if let Err(e) = result {
                eprintln!("Failed to write ISO: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("Detected: Linux ISO");
        io::stdout().flush().ok();
        let cluster_bytes: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4096);
        let result = linux_flow::write_iso_to_usb_stream(
            iso_path, usb_device, cluster_bytes
        );
        if let Err(e) = result {
            eprintln!("Failed to write ISO: {}", e);
            std::process::exit(1);
        }
    }
    println!("Done!");
    io::stdout().flush().ok();
}
