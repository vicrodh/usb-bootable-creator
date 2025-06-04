// Utility functions for dependency checks and privilege escalation

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use libc; // For geteuid
use serde_json; // For JSON parsing
use which; // To check if a binary exists

/// Utility: Check if running as root
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Utility: Relaunch with pkexec if not root
pub fn ensure_root() {
    if !is_root() {
        let exe = std::env::current_exe().unwrap();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut cmd = Command::new("pkexec");
        cmd.arg(exe);
        for arg in args {
            cmd.arg(arg);
        }
        let _ = cmd.status();
        std::process::exit(0);
    }
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

    let mountpt = "/tmp/iso_detect";
    let _ = fs::create_dir_all(mountpt);

    // Try mounting as user (no pkexec)
    let mount_result = Command::new("mount")
        .arg("-o")
        .arg("loop,ro")
        .arg(iso_path)
        .arg(mountpt)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    // Small delay to allow mount to complete
    sleep(Duration::from_millis(200));

    let result = match mount_result {
        Ok(status) if status.success() => {
            let bootmgr = Path::new(mountpt).join("bootmgr");
            let sources = Path::new(mountpt).join("sources");
            let is_win = bootmgr.is_file() && sources.is_dir();

            let _ = Command::new("umount").arg(mountpt).status();
            let _ = fs::remove_dir(mountpt);
            Some(is_win)
        }
        Ok(_status) => {
            // Non-successful exit (e.g. permission denied)
            let _ = Command::new("umount").arg(mountpt).status();
            let _ = fs::remove_dir(mountpt);
            None
        }
        Err(_) => {
            let _ = Command::new("umount").arg(mountpt).status();
            let _ = fs::remove_dir(mountpt);
            None
        }
    };

    result
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
