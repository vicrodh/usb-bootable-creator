// Utility functions for dependency checks and privilege escalation

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use libc; // For geteuid
use serde_json; // For JSON parsing
use which; // To check if a binary exists

/// Parse an rsync `--info=progress2` line and return (bytes_transferred, speed_mb_per_s).
pub fn parse_rsync_progress(line: &str) -> Option<(u64, Option<f64>)> {
    let trimmed = line.trim_start();
    let mut parts = trimmed.split_whitespace();

    let bytes_str = parts.next()?;
    let bytes = bytes_str.replace(',', "").parse::<u64>().ok()?;

    let speed_token = parts.find(|p| p.contains("/s"));
    let speed_mb = speed_token.and_then(|token| {
        let cleaned = token.replace("/s", "");
        if let Some(value) = cleaned.strip_suffix("MB") {
            value.parse::<f64>().ok()
        } else if let Some(value) = cleaned.strip_suffix("MiB") {
            value.parse::<f64>().ok()
        } else if let Some(value) = cleaned.strip_suffix("kB") {
            value.parse::<f64>().ok().map(|v| v / 1024.0)
        } else if let Some(value) = cleaned.strip_suffix("KB") {
            value.parse::<f64>().ok().map(|v| v / 1024.0)
        } else {
            None
        }
    });

    Some((bytes, speed_mb))
}

/// Detect if a device path refers to a USB device via lsblk transport.
pub fn is_usb_device(device: &str) -> bool {
    let dev_name = device.trim_start_matches("/dev/");
    if let Ok(output) = Command::new("lsblk").args(["-ndo", "TRAN", dev_name]).output() {
        let tran = String::from_utf8_lossy(&output.stdout).to_lowercase();
        return tran.contains("usb");
    }
    false
}

/// Detect the optimal (physical) block size for a device. Falls back to 4096 on errors.
pub fn get_device_optimal_block_size(device: &str) -> io::Result<u64> {
    let dev_name = device.trim_start_matches("/dev/");
    let path = format!("/sys/block/{}/queue/physical_block_size", dev_name);
    let contents = fs::read_to_string(&path)?;
    let size = contents.trim().parse::<u64>().unwrap_or(4096);
    Ok(size.max(512))
}

/// Check if ntfs-3g is available on the system.
pub fn has_ntfs3g() -> bool {
    Command::new("which")
        .arg("ntfs-3g")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Utility: Check if running as root
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Utility: Check if running inside Flatpak
pub fn is_flatpak() -> bool {
    // Check for the Flatpak info file
    std::path::Path::new("/.flatpak-info").exists() ||
    // Check for Flatpak environment variables
    std::env::var("FLATPAK_ID").is_ok() ||
    std::env::var("FLATPAK_SANDBOX_DIR").is_ok()
}

/// Utility: Get the original user's home directory
pub fn get_user_home() -> String {
    // Try to get the original home directory preserved during elevation
    if let Ok(home) = std::env::var("ORIGINAL_HOME") {
        return home;
    }

    // Fallback to current HOME (should work in most cases)
    if let Ok(home) = std::env::var("HOME") {
        return home;
    }

    // Final fallback - try to get user's home from /etc/passwd
    let user = get_original_user();
    if let Ok(output) = std::process::Command::new("getent")
            .arg("passwd")
            .arg(&user)
            .output()
        {
            let passwd_line = String::from_utf8_lossy(&output.stdout);
            for line in passwd_line.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 6 {
                    return parts[5].to_string();
                }
            }
        }

    // Last resort
    "/".to_string()
}

/// Utility: Get the original username
pub fn get_original_user() -> String {
    if let Ok(user) = std::env::var("ORIGINAL_USER") {
        return user;
    }

    // Try current user
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

/// Utility: Apply user's visual theme settings
pub fn apply_user_theme() {
    // Apply GTK theme if specified
    if let Ok(theme) = std::env::var("GTK_THEME") {
        unsafe { std::env::set_var("GTK_THEME", theme); }
    }

    // Apply icon theme if available
    if let Ok(icon_theme) = std::env::var("ICON_THEME") {
        unsafe { std::env::set_var("ICON_THEME", icon_theme); }
    }

    // Try to load theme from user's config directory
    let xdg_config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = get_user_home();
        format!("{}/.config", home)
    });

    let gtk_settings_path = format!("{}/gtk-3.0/settings.ini", xdg_config);
    if std::path::Path::new(&gtk_settings_path).exists() {
        // GTK should automatically pick up these settings
        unsafe { std::env::set_var("GTK_SETTINGS_PATH", &gtk_settings_path); }
    }
}

/// Utility: Check if root permissions are needed and how to handle
pub fn check_root_requirements() -> (bool, bool) {
    let needs_root = !is_root();
    let is_flatpak_env = is_flatpak();
    (needs_root, is_flatpak_env)
}

/// Utility: Show Flatpak-specific permission request dialog (call after GTK init)
pub fn show_flatpak_permission_dialog() {
    use gtk4::prelude::*;
    use gtk4::{MessageDialog, ButtonsType, MessageType, ResponseType, Window};

    // Create a simple temporary window as parent
    let temp_window = Window::builder()
        .title("MajUSB")
        .default_width(400)
        .default_height(200)
        .build();

    let dialog = MessageDialog::builder()
        .text("Root Permissions Required")
        .secondary_text(
            "This application needs root access to manage USB devices.\n\n\
            When running in Flatpak, please use:\n\
            flatpak-spawn --host pkexec flatpak run com.github.vicrodh.MajUSB\n\n\
            Or launch from terminal with the above command."
        )
        .buttons(ButtonsType::Ok)
        .message_type(MessageType::Warning)
        .modal(true)
        .transient_for(&temp_window)
        .build();

    dialog.set_default_response(ResponseType::Ok);

    dialog.connect_response(move |dialog, _| {
        dialog.close();
    });

    temp_window.show();
    dialog.show();
}

/// Utility: Relaunch with pkexec if not root (normal execution only)
pub fn ensure_root_normal() {
    if !is_root() && !is_flatpak() {
        let exe = std::env::current_exe().unwrap();
        let args: Vec<String> = std::env::args().skip(1).collect();

        // Propaga variables de entorno gráficas y del usuario
        let display = std::env::var("DISPLAY").unwrap_or_default();
        let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
        let xauth = std::env::var("XAUTHORITY").unwrap_or_default();
        let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();

        // Variables del entorno del usuario original
        let user_home = std::env::var("HOME").unwrap_or_default();
        let user = std::env::var("USER").unwrap_or_else(|_| std::env::var("USERNAME").unwrap_or_default());
        let xdg_data_home = std::env::var("XDG_DATA_HOME").unwrap_or_default();
        let xdg_config_home = std::env::var("XDG_CONFIG_HOME").unwrap_or_default();
        let gtk_theme = std::env::var("GTK_THEME").unwrap_or_default();
        let icon_theme = std::env::var("ICON_THEME").unwrap_or_default();

        let mut cmd = std::process::Command::new("pkexec");
        cmd.arg("env");

        // Variables gráficas
        if !display.is_empty() {
            cmd.arg(format!("DISPLAY={}", display));
        }
        if !wayland.is_empty() {
            cmd.arg(format!("WAYLAND_DISPLAY={}", wayland));
        }
        if !xauth.is_empty() {
            cmd.arg(format!("XAUTHORITY={}", xauth));
        }
        if !xdg_runtime.is_empty() {
            cmd.arg(format!("XDG_RUNTIME_DIR={}", xdg_runtime));
        }

        // Variables del entorno del usuario original
        if !user_home.is_empty() {
            cmd.arg(format!("ORIGINAL_HOME={}", user_home));
            cmd.arg(format!("HOME={}", user_home)); // Mantener HOME original
        }
        if !user.is_empty() {
            cmd.arg(format!("ORIGINAL_USER={}", user));
        }
        if !xdg_data_home.is_empty() {
            cmd.arg(format!("XDG_DATA_HOME={}", xdg_data_home));
        }
        if !xdg_config_home.is_empty() {
            cmd.arg(format!("XDG_CONFIG_HOME={}", xdg_config_home));
        }
        if !gtk_theme.is_empty() {
            cmd.arg(format!("GTK_THEME={}", gtk_theme));
        }
        if !icon_theme.is_empty() {
            cmd.arg(format!("ICON_THEME={}", icon_theme));
        }
        cmd.arg(exe);
        for arg in args {
            cmd.arg(arg);
        }
        let _ = cmd.status();
        std::process::exit(0);
    }
}

/// Legacy function for compatibility
pub fn ensure_root() {
    ensure_root_normal();
}

/// Utility: List block devices (USB drives)
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

/// Detect if the ISO is a Windows installer by mounting and checking for Windows-specific files.
/// Returns Some(true) if Windows ISO, Some(false) if Linux ISO, None if detection failed (e.g. permission denied)
pub fn is_windows_iso(iso_path: &str) -> Option<bool> {
    use std::thread::sleep;
    use std::time::Duration;
    use std::fs;
    use std::path::Path;

    // Use udisksctl to mount as user/root
    let mount_output = Command::new("udisksctl")
        .arg("loop-setup")
        .arg("-f")
        .arg(iso_path)
        .output()
        .ok()?;
    if !mount_output.status.success() {
        return None;
    }
    // Parse device path from output: "Mapped file ... as /dev/loopX."
    let stdout = String::from_utf8_lossy(&mount_output.stdout);
    let dev_line = stdout.lines().find(|l| l.contains("/dev/loop"))?;
    let dev_path = dev_line.split_whitespace().last()?.trim_end_matches('.');

    // Mount the loop device
    let mount_dir = tempfile::tempdir().ok()?;
    let mount_status = Command::new("mount")
        .arg(dev_path)
        .arg(mount_dir.path())
        .output()
        .ok()?;
    if !mount_status.status.success() {
        // Clean up loop device
        let _ = Command::new("udisksctl").arg("loop-delete").arg("-b").arg(dev_path).status();
        return None;
    }
    sleep(Duration::from_millis(200));
    let mount_point = mount_dir.path();

    // Check for Windows files
    let bootmgr = mount_point.join("bootmgr");
    let sources = mount_point.join("sources");
    if bootmgr.is_file() && sources.is_dir() {
        // Unmount and clean up
        let _ = Command::new("umount").arg(mount_point).status();
        let _ = Command::new("udisksctl").arg("loop-delete").arg("-b").arg(dev_path).status();
        return Some(true); // Windows ISO
    }

    // Check for Linux markers (must match at least one directory or file)
    let linux_markers = [
        "boot", "casper", "syslinux", "isolinux", "EFI", "live", "kernel", "initrd", "vmlinuz", "arch", "loader", "install", "preseed", "dists", "pool", ".disk", "filesystem.squashfs"
    ];
    let found_linux = linux_markers.iter().any(|m| mount_point.join(m).exists());

    // Unmount and clean up
    let _ = Command::new("umount").arg(mount_point).status();
    let _ = Command::new("udisksctl").arg("loop-delete").arg("-b").arg(dev_path).status();

    if found_linux {
        Some(false) // Linux ISO
    } else {
        None // Unknown or not a bootable ISO
    }
}

/// Utility: Check for required system packages
///
/// This function checks if the necessary binaries are available (`lsblk`, `dd`, `mkfs.vfat`,
/// `mkfs.ntfs`, `parted`, `wipefs`, `mount`, `umount`, `rsync`). If any are missing, it maps
/// those missing binaries to the actual package names for the detected Linux distribution,
/// then returns an `Option` containing a vector of missing package names and a single string
/// with the appropriate installation command for that distribution.
///
/// Returns:
/// - `None` if all required binaries are present.
/// - `Some((Vec<String>, String))` where:
///    * `Vec<String>` is the list of missing package names to install.
///    * `String` is the installation command the user can copy-paste.
///
/// Supported distributions:
/// - Arch Linux (and Manjaro, EndeavourOS, Artix, Garuda, SteamOS)
/// - Debian (and MX Linux, antiX)
/// - Ubuntu (and its flavors like Linux Mint, Pop!_OS, etc.)
/// - Fedora (and Nobara, Bazzite)
/// - openSUSE (Leap, Tumbleweed, GeckoLinux)
/// - Alpine Linux
/// - Void Linux
/// - Gentoo
/// - NixOS
pub fn check_required_packages() -> Option<(Vec<String>, String)> {
    // List of required command names
    let required_bins = vec![
        "lsblk",
        "dd",
        "mkfs.vfat",
        "mkfs.ntfs",
        "parted",
        "sgdisk",
        "wipefs",
        "mount",
        "umount",
        "rsync",
    ];

    // Map each binary to the package name per distribution
    let mut pkg_map: HashMap<&str, HashMap<&str, &str>> = HashMap::new();

    // ARCH Linux & derivatives
    let mut arch_map = HashMap::new();
    arch_map.insert("lsblk", "util-linux");
    arch_map.insert("dd", "coreutils");
    arch_map.insert("mkfs.vfat", "dosfstools");
    arch_map.insert("mkfs.ntfs", "ntfs-3g");
    arch_map.insert("parted", "parted");
    arch_map.insert("sgdisk", "gptfdisk");
    arch_map.insert("wipefs", "util-linux");
    arch_map.insert("mount", "util-linux");
    arch_map.insert("umount", "util-linux");
    arch_map.insert("rsync", "rsync");
    pkg_map.insert("arch", arch_map);

    // Debian & derivatives
    let mut debian_map = HashMap::new();
    debian_map.insert("lsblk", "util-linux");
    debian_map.insert("dd", "coreutils");
    debian_map.insert("mkfs.vfat", "dosfstools");
    debian_map.insert("mkfs.ntfs", "ntfs-3g");
    debian_map.insert("parted", "parted");
    debian_map.insert("sgdisk", "gdisk");
    debian_map.insert("wipefs", "util-linux");
    debian_map.insert("mount", "mount");
    debian_map.insert("umount", "mount");
    debian_map.insert("rsync", "rsync");
    pkg_map.insert("debian", debian_map);

    // Ubuntu & derivatives
    let mut ubuntu_map = HashMap::new();
    ubuntu_map.insert("lsblk", "util-linux");
    ubuntu_map.insert("dd", "coreutils");
    ubuntu_map.insert("mkfs.vfat", "dosfstools");
    ubuntu_map.insert("mkfs.ntfs", "ntfs-3g");
    ubuntu_map.insert("parted", "parted");
    ubuntu_map.insert("sgdisk", "gdisk");
    ubuntu_map.insert("wipefs", "util-linux");
    ubuntu_map.insert("mount", "util-linux");
    ubuntu_map.insert("umount", "util-linux");
    ubuntu_map.insert("rsync", "rsync");
    pkg_map.insert("ubuntu", ubuntu_map);

    // Fedora & derivatives
    let mut fedora_map = HashMap::new();
    fedora_map.insert("lsblk", "util-linux");
    fedora_map.insert("dd", "coreutils");
    fedora_map.insert("mkfs.vfat", "dosfstools");
    fedora_map.insert("mkfs.ntfs", "ntfs-3g");
    fedora_map.insert("parted", "parted");
    fedora_map.insert("sgdisk", "gdisk");
    fedora_map.insert("wipefs", "util-linux");
    fedora_map.insert("mount", "util-linux");
    fedora_map.insert("umount", "util-linux");
    fedora_map.insert("rsync", "rsync");
    pkg_map.insert("fedora", fedora_map);

    // openSUSE & derivatives
    let mut opensuse_map = HashMap::new();
    opensuse_map.insert("lsblk", "util-linux");
    opensuse_map.insert("dd", "coreutils");
    opensuse_map.insert("mkfs.vfat", "dosfstools");
    opensuse_map.insert("mkfs.ntfs", "ntfs-3g");
    opensuse_map.insert("parted", "parted");
    opensuse_map.insert("sgdisk", "gptfdisk");
    opensuse_map.insert("wipefs", "util-linux");
    opensuse_map.insert("mount", "util-linux");
    opensuse_map.insert("umount", "util-linux");
    opensuse_map.insert("rsync", "rsync");
    pkg_map.insert("opensuse", opensuse_map);

    // Alpine Linux
    let mut alpine_map = HashMap::new();
    alpine_map.insert("lsblk", "lsblk");
    alpine_map.insert("dd", "coreutils"); // coreutils provides full dd; BusyBox has limited dd by default
    alpine_map.insert("mkfs.vfat", "dosfstools");
    alpine_map.insert("mkfs.ntfs", "ntfs-3g-progs"); // provides mkfs.ntfs
    alpine_map.insert("parted", "parted");
    alpine_map.insert("sgdisk", "gptfdisk");
    alpine_map.insert("wipefs", "wipefs");
    alpine_map.insert("mount", "mount");
    alpine_map.insert("umount", "umount");
    alpine_map.insert("rsync", "rsync");
    pkg_map.insert("alpine", alpine_map);

    // Void Linux
    let mut void_map = HashMap::new();
    void_map.insert("lsblk", "util-linux");
    void_map.insert("dd", "coreutils");
    void_map.insert("mkfs.vfat", "dosfstools");
    void_map.insert("mkfs.ntfs", "ntfs-3g");
    void_map.insert("parted", "parted");
    void_map.insert("sgdisk", "gptfdisk");
    void_map.insert("wipefs", "util-linux");
    void_map.insert("mount", "util-linux");
    void_map.insert("umount", "util-linux");
    void_map.insert("rsync", "rsync");
    pkg_map.insert("void", void_map);

    // Gentoo
    let mut gentoo_map = HashMap::new();
    gentoo_map.insert("lsblk", "sys-apps/util-linux");
    gentoo_map.insert("dd", "sys-apps/coreutils");
    gentoo_map.insert("mkfs.vfat", "sys-fs/dosfstools");
    gentoo_map.insert("mkfs.ntfs", "sys-fs/ntfs3g"); // requires USE flag ntfsprogs
    gentoo_map.insert("parted", "sys-block/parted");
    gentoo_map.insert("sgdisk", "sys-apps/gptfdisk");
    gentoo_map.insert("wipefs", "sys-apps/util-linux");
    gentoo_map.insert("mount", "sys-apps/util-linux");
    gentoo_map.insert("umount", "sys-apps/util-linux");
    gentoo_map.insert("rsync", "net-misc/rsync");
    pkg_map.insert("gentoo", gentoo_map);

    // NixOS
    let mut nixos_map = HashMap::new();
    nixos_map.insert("lsblk", "nixos.util-linux");
    nixos_map.insert("dd", "nixos.coreutils");
    nixos_map.insert("mkfs.vfat", "nixos.dosfstools");
    nixos_map.insert("mkfs.ntfs", "nixos.ntfs3g");
    nixos_map.insert("parted", "nixos.parted");
    nixos_map.insert("sgdisk", "nixos.gptfdisk");
    nixos_map.insert("wipefs", "nixos.util-linux");
    nixos_map.insert("mount", "nixos.util-linux");
    nixos_map.insert("umount", "nixos.util-linux");
    nixos_map.insert("rsync", "nixos.rsync");
    pkg_map.insert("nixos", nixos_map);

    // Read /etc/os-release to detect distribution
    let content = fs::read_to_string("/etc/os-release").unwrap_or_default().to_lowercase();
    let distro = if content.contains("id=arch")
        || content.contains("id=manjaro")
        || content.contains("id=endeavouros")
        || content.contains("id=artix")
        || content.contains("id=garuda")
        || content.contains("id=steamos")
    {
        "arch"
    } else if content.contains("id=fedora")
        || content.contains("id=nobara")
        || content.contains("id=bazzite")
    {
        "fedora"
    } else if content.contains("id=ubuntu")
        || content.contains("id=pop")
        || content.contains("id=elementary")
        || content.contains("id=zorin")
        || content.contains("id=kubuntu")
        || content.contains("id=lubuntu")
        || content.contains("id=xubuntu")
        || content.contains("id=linuxmint")
    {
        "ubuntu"
    } else if content.contains("id=debian")
        || content.contains("id=mx")
        || content.contains("id=antix")
    {
        "debian"
    } else if content.contains("id=suse")
        || content.contains("id=opensuse")
        || content.contains("id=geckolinux")
    {
        "opensuse"
    } else if content.contains("id=alpine")
    {
        "alpine"
    } else if content.contains("id=void")
    {
        "void"
    } else if content.contains("id=gentoo")
    {
        "gentoo"
    } else if content.contains("id=nixos")
    {
        "nixos"
    } else {
        "other"
    };

    // For "other" distributions, we fallback to using the binary names as package names
    // and a generic instruction
    let default_pkg_map: HashMap<&str, &str> = required_bins
        .iter()
        .map(|&b| (b, b))
        .collect();

    // Collect missing packages (deduplicated and sorted)
    use std::collections::BTreeSet;
    let mut missing_pkgs: BTreeSet<String> = BTreeSet::new();
    for &bin in &required_bins {
        if which::which(bin).is_err() {
            let pkg_name = pkg_map
                .get(distro)
                .and_then(|m| m.get(bin))
                .copied()
                .unwrap_or_else(|| default_pkg_map.get(bin).copied().unwrap_or(bin));
            missing_pkgs.insert(pkg_name.to_string());
        }
    }
    if missing_pkgs.is_empty() {
        return None; // All required binaries are present
    }
    let pkgs_str = missing_pkgs.iter().cloned().collect::<Vec<_>>().join(" ");
    // Construct the installation command based on distribution
    let install_cmd = match distro {
        "arch" => format!("sudo pacman -S --needed {}", pkgs_str),
        "fedora" => format!("sudo dnf install -y {}", pkgs_str),
        "ubuntu" => format!("sudo apt update && sudo apt install -y {}", pkgs_str),
        "debian" => format!("sudo apt update && sudo apt install -y {}", pkgs_str),
        "opensuse" => format!("sudo zypper install -y {}", pkgs_str),
        "alpine" => format!("sudo apk add {}", pkgs_str),
        "void" => format!("sudo xbps-install -S {}", pkgs_str),
        "gentoo" => format!("sudo emerge --ask {}", pkgs_str),
        "nixos" => {
            let joined = missing_pkgs
                .iter()
                .map(|p| p.replace("nixos.", ""))
                .collect::<Vec<_>>()
                .join(" ");
            format!("nix-env -iA nixos.{}", joined)
        }
        _ => format!("Please install: {}", missing_pkgs.iter().cloned().collect::<Vec<_>>().join(", ")),
    };
    Some((missing_pkgs.iter().cloned().collect(), install_cmd))
}

#[cfg(test)]
mod tests {
    use super::parse_rsync_progress;

    #[test]
    fn parses_rsync_progress_line_with_speed() {
        let line = "  123,456,789  45%  12.3MB/s    0:10:00 (xfr#5, to-chk=0/1)";
        let parsed = parse_rsync_progress(line).unwrap();
        assert_eq!(parsed.0, 123_456_789);
        assert_eq!(parsed.1.unwrap_or(0.0), 12.3);
    }

    #[test]
    fn parses_rsync_progress_line_without_speed() {
        let line = "  50,000,000  10%   0:05:00 (xfr#1, to-chk=4/5)";
        let parsed = parse_rsync_progress(line).unwrap();
        assert_eq!(parsed.0, 50_000_000);
        assert!(parsed.1.is_none());
    }
}
