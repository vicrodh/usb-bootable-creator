// Utility functions for dependency checks and privilege escalation

use std::process::Command;
use std::fs;
use std::path::Path;
use libc; // Add this import for geteuid
use serde_json; // Add this import for JSON parsing

// Utility: Check if running as root
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

// Utility: Relaunch with pkexec if not root
pub fn ensure_root() {
    if !is_root() {
        let exe = std::env::current_exe().unwrap();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut cmd = Command::new("pkexec");
        cmd.arg(exe);
        for arg in args { cmd.arg(arg); }
        let _ = cmd.status();
        std::process::exit(0);
    }
}

// Utility: List block devices (USB drives)
pub fn list_usb_devices() -> Vec<(String, String)> {
    // Use lsblk to get device name and model
    let output = Command::new("lsblk")
        .args(["-d", "-o", "NAME,MODEL,TRAN,SIZE,TYPE", "-J"])
        .output()
        .expect("lsblk failed");
    let json = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
        if let Some(blockdevices) = parsed["blockdevices"].as_array() {
            for dev in blockdevices {
                if dev["type"] == "disk" && dev["tran"] == "usb" {
                    let name = dev["name"].as_str().unwrap_or("");
                    let model = dev["model"].as_str().unwrap_or("");
                    let size = dev["size"].as_str().unwrap_or("");
                    devices.push((format!("/dev/{}", name), format!("{} {}", model, size)));
                }
            }
        }
    }
    devices
}

// Detect if the ISO is a Windows installer by mounting and checking for Windows-specific files.
/// Returns Some(true) if Windows ISO, Some(false) if Linux ISO, None if detection failed (e.g. permission denied)
pub fn is_windows_iso(iso_path: &str) -> Option<bool> {
    use std::thread::sleep;
    use std::time::Duration;
    let mountpt = "/tmp/iso_detect";
    let _ = fs::create_dir_all(mountpt);
    // Try mounting as user (no pkexec)
    let mount_result = Command::new("mount")
        .arg("-o").arg("loop,ro")
        .arg(iso_path)
        .arg(mountpt)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    sleep(Duration::from_millis(200));
    let result = match mount_result {
        Ok(status) if status.success() => {
            let bootmgr = Path::new(mountpt).join("bootmgr");
            let sources = Path::new(mountpt).join("sources");
            let is_win = bootmgr.is_file() && sources.is_dir();
            let _ = Command::new("umount").arg(mountpt).status();
            let _ = fs::remove_dir(mountpt);
            Some(is_win)
        },
        Ok(_status) => {
            // Non-successful exit (e.g. permission denied)
            let _ = Command::new("umount").arg(mountpt).status();
            let _ = fs::remove_dir(mountpt);
            None
        },
        Err(_) => {
            let _ = Command::new("umount").arg(mountpt).status();
            let _ = fs::remove_dir(mountpt);
            None
        }
    };
    result
}
