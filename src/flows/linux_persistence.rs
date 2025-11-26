//! Linux persistence support for USB bootable drives

use crate::error::{UsbCreatorError, UsbCreatorResult};
use scopeguard;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile;

const SAFETY_MARGIN_MB: u64 = 512;
const TABLE_REFRESH_ATTEMPTS: usize = 5;

/// Configuration for Linux persistence
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Whether persistence is enabled
    pub enabled: bool,
    /// Size of persistence partition in MB
    pub size_mb: u64,
    /// Type of persistence (casper, overlayfs, etc.)
    pub persistence_type: PersistenceType,
    /// Label for persistence partition
    pub label: String,
    /// Desired partition table type (GPT or MBR)
    pub partition_table: PartitionTableType,
}

/// Types of persistence support
#[derive(Debug, Clone)]
pub enum PersistenceType {
    /// Ubuntu/Debian Casper persistence
    Casper,
    /// OverlayFS-based persistence
    OverlayFS,
    /// Custom persistence method
    Custom(String),
}

/// Supported partition table types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionTableType {
    Gpt,
    Mbr,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            size_mb: 4096, // 4GB default
            persistence_type: PersistenceType::Casper,
            label: "persistence".to_string(),
            partition_table: PartitionTableType::Gpt,
        }
    }
}

/// Create persistence partition on USB drive after ISO writing
pub fn create_persistence_partition(
    usb_device: &str,
    config: &PersistenceConfig,
) -> UsbCreatorResult<()> {
    if !config.enabled {
        return Ok(());
    }

    println!("[PERSISTENCE] Creating {}MB persistence partition...", config.size_mb);

    // Ensure kernel has flushed caches and re-read partition table after dd
    let _ = Command::new("sync").status();
    let _ = Command::new("partprobe").arg(usb_device).status();
    settle_udev();
    thread::sleep(Duration::from_millis(500));
    refresh_partition_table(usb_device)?;

    // Detect existing partition table and ensure it matches requested type
    let current_table = detect_partition_table_type(usb_device)?;
    if current_table != config.partition_table {
        return Err(UsbCreatorError::validation_error(format!(
            "Requested partition table {:?} but device currently uses {:?}. Select the matching table type for this ISO or recreate the media.",
            config.partition_table, current_table
        )));
    }

    // Ensure nothing is mounted from the target device before we repartition
    let previously_mounted = unmount_device_partitions(usb_device)?;
    // Keep remount best-effort but do not let it mask failures; only if we actually unmounted something
    let _remount_guard = scopeguard::guard(previously_mounted, |mounts: Vec<(String, String)>| {
        if mounts.is_empty() {
            return;
        }
        println!("[PERSISTENCE] Remounting previously mounted partitions (best effort)...");
        for (dev, mp) in mounts {
            println!("[PERSISTENCE] Remounting {} to {}", dev, mp);
            let _ = Command::new("mount").args([dev.as_str(), mp.as_str()]).status();
        }
    });

    // For GPT-based ISOs, expand the secondary GPT to the end of the device so new partitions fit
    maybe_expand_gpt(usb_device)?;
    let _ = run_command("partprobe", &[usb_device]);
    settle_udev();
    thread::sleep(Duration::from_millis(500));
    refresh_partition_table(usb_device)?;

    // Find the next available partition number
    let partition_number = find_next_partition_number(usb_device)?;
    let partition_path = build_partition_path(usb_device, partition_number);

    // Calculate partition start (we need to find where the existing partitions end)
    let start_sector = find_next_available_sector(usb_device)?;
    let total_sectors = get_total_sectors(usb_device)?;
    ensure_free_space(usb_device, start_sector, total_sectors, config.size_mb)?;
    let end_sector = start_sector + (config.size_mb * 2048).saturating_sub(1); // 512-byte sectors

    // One more settle before creating the partition to avoid racing table updates
    let _ = Command::new("sync").status();
    let _ = run_command("partprobe", &[usb_device]);
    settle_udev();
    thread::sleep(Duration::from_millis(300));

    println!("[PERSISTENCE] Creating new partition {} ({}s-{}s)...", partition_number, start_sector, end_sector);

    // Create new partition
    if let Err(e) = run_command("parted", &[
        "-s", usb_device, "mkpart", "primary",
        &format!("{}s", start_sector),
        &format!("{}s", end_sector)
    ]) {
        println!("[PERSISTENCE] ERROR while creating partition: {}", e);
        return Err(e);
    }

    // Set partition flag
    if config.partition_table == PartitionTableType::Mbr {
        println!("[PERSISTENCE] Marking partition {} as LBA (MBR)...", partition_number);
        if let Err(e) = run_command("parted", &[
            "-s", usb_device, "set", &partition_number.to_string(), "lba", "on"
        ]) {
            println!("[PERSISTENCE] ERROR while setting partition flag: {}", e);
            return Err(e);
        }
    } else {
        println!("[PERSISTENCE] GPT detected; skipping LBA flag (not applicable).");
    }

    println!("[PERSISTENCE] Formatting persistence partition as ext4...");
    if let Err(e) = run_command("mkfs.ext4", &[
        "-L", &config.label,
        "-F",  // Force creation
        &partition_path
    ]) {
        println!("[PERSISTENCE] ERROR while formatting persistence partition: {}", e);
        return Err(e);
    }

    // Add overlay kernel param for Fedora-style overlay if applicable
    if matches!(config.persistence_type, PersistenceType::OverlayFS) && matches!(config.partition_table, PartitionTableType::Gpt | PartitionTableType::Mbr) {
        inject_overlay_kernel_params(usb_device, &config.label);
    }

    // Final settle to make the new partition visible
    let _ = Command::new("sync").status();
    let _ = run_command("partprobe", &[usb_device]);
    settle_udev();

    println!("[PERSISTENCE] Setting up persistence configuration...");

    // Configure persistence based on type
    match &config.persistence_type {
        PersistenceType::Casper => setup_casper_persistence(&partition_path, config)?,
        PersistenceType::OverlayFS => setup_overlayfs_persistence(&partition_path, config)?,
        PersistenceType::Custom(method) => setup_custom_persistence(&partition_path, config, method)?,
    }

    // Refresh partition table so the OS sees the new partition
    let _ = run_command("partprobe", &[usb_device]);

    println!("Linux persistence setup completed successfully!");
    Ok(())
}

/// Find the next available partition number for a device
fn find_next_partition_number(device: &str) -> UsbCreatorResult<u32> {
    let output = run_command_with_output("lsblk", &["-ln", "-o", "NAME", device])?;
    let device_name = device.trim_start_matches("/dev/");
    let mut max_number = 0;

    for line in output.lines() {
        let name = line.trim();
        if name == device_name {
            continue;
        }
        if name.starts_with(device_name) {
            let suffix = name.trim_start_matches(device_name).trim_start_matches('p');
            if let Ok(num) = suffix.parse::<u32>() {
                max_number = max_number.max(num);
            }
        }
    }

    Ok(max_number + 1)
}

/// Find the next available sector for partition creation
fn find_next_available_sector(device: &str) -> UsbCreatorResult<u64> {
    let output = run_command_with_output("parted", &[
        "-ms", device, "unit", "s", "print"
    ])?;

    let mut max_sector = 2048; // Start after first MB

    for line in output.lines() {
        if line.starts_with(|c: char| c.is_ascii_digit()) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let end_raw = parts[2].trim_end_matches('s');
                if let Ok(end_sector) = end_raw.parse::<u64>() {
                    max_sector = max_sector.max(end_sector);
                }
            }
        }
    }

    Ok(max_sector + 1) // Start from next sector
}

/// Get total sectors of device via blockdev --getsz
fn get_total_sectors(device: &str) -> UsbCreatorResult<u64> {
    let output = run_command_with_output("blockdev", &["--getsz", device])?;
    let sectors = output.trim().parse::<u64>()?;
    Ok(sectors)
}

/// Detect current partition table type via parted -ms print
fn detect_partition_table_type(device: &str) -> UsbCreatorResult<PartitionTableType> {
    let output = run_command_with_output("parted", &["-ms", device, "unit", "s", "print"])?;
    for line in output.lines() {
        if line.starts_with("/dev/") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 6 {
                let label = parts[5].to_lowercase();
                if label.contains("gpt") {
                    return Ok(PartitionTableType::Gpt);
                } else if label.contains("msdos") {
                    return Ok(PartitionTableType::Mbr);
                }
            }
        }
    }
    Err(UsbCreatorError::validation_error(
        "Could not detect partition table type for device",
    ))
}

/// Build partition path that works for /dev/sdX and /dev/nvmeXpY devices
fn build_partition_path(device: &str, partition_number: u32) -> String {
    if device.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("{}p{}", device, partition_number)
    } else {
        format!("{}{}", device, partition_number)
    }
}

/// Unmount any mounted partitions from the target device to avoid busy errors.
/// Returns the list of (device, mountpoint) that were unmounted so they can be restored.
fn unmount_device_partitions(device: &str) -> UsbCreatorResult<Vec<(String, String)>> {
    println!("[PERSISTENCE] Checking for mounted partitions on {}...", device);
    let output = run_command_with_output("lsblk", &["-ln", "-o", "NAME,MOUNTPOINT", device])?;
    let mut unmounted = false;
    let mut mounts: Vec<(String, String)> = Vec::new();
    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap_or_default();
        let mount_point = parts.next();
        if let Some(mp) = mount_point {
            let dev_path = format!("/dev/{}", name);
            println!("[PERSISTENCE] Unmounting {} from {}", dev_path, mp);
            let _ = run_command("umount", &[mp]);
            unmounted = true;
            mounts.push((dev_path, mp.to_string()));
        }
    }
    if !unmounted {
        println!("[PERSISTENCE] No mounted partitions detected on {}.", device);
    }
    // Give the kernel/udev a moment to release the device
    settle_udev();
    thread::sleep(Duration::from_millis(200));
    Ok(mounts)
}

/// Ensure free space is sufficient for the requested persistence size plus a safety margin.
fn ensure_free_space(device: &str, start_sector: u64, total_sectors: u64, size_mb: u64) -> UsbCreatorResult<()> {
    // sectors are 512 bytes
    let free_sectors = total_sectors.saturating_sub(start_sector);
    let free_mb = free_sectors.saturating_mul(512) / 1024 / 1024;
    if free_mb <= SAFETY_MARGIN_MB {
        return Err(UsbCreatorError::validation_error(
            format!("Not enough free space on {} for persistence (only {} MB free)", device, free_mb),
        ));
    }
    let needed = size_mb + SAFETY_MARGIN_MB;
    if free_mb < needed {
        return Err(UsbCreatorError::validation_error(
            format!(
                "Persistence size {} MB exceeds available space {} MB ({} MB safety margin)",
                size_mb, free_mb, SAFETY_MARGIN_MB
            ),
        ));
    }
    Ok(())
}

/// Refresh partition table with retries to avoid races right after dd
fn refresh_partition_table(device: &str) -> UsbCreatorResult<()> {
    for attempt in 1..=TABLE_REFRESH_ATTEMPTS {
        println!("[PERSISTENCE] Refreshing partition table (attempt {}/{})...", attempt, TABLE_REFRESH_ATTEMPTS);
        let _ = Command::new("sync").status();
        let _ = Command::new("partprobe").arg(device).status();
        settle_udev();
        thread::sleep(Duration::from_millis(300));
        // Probe with parted print; success means kernel sees the table
        match run_command_with_output("parted", &["-ms", device, "unit", "s", "print"]) {
            Ok(_) => return Ok(()),
            Err(e) => {
                // Ignore common 2048/512 warning as non-fatal and continue
                if format!("{}", e).contains("physical block size is 2048 bytes, but Linux says it is 512 bytes") {
                    println!("[PERSISTENCE] Parted reported 2048/512 block-size warning; continuing.");
                    return Ok(());
                }
                println!("[PERSISTENCE] Partition table refresh not yet visible: {}. Retrying...", e);
            }
        }
    }
    Err(UsbCreatorError::validation_error(
        "Kernel did not refresh partition table after write; aborting persistence creation",
    ))
}
/// Try to relocate the GPT backup header to the end of the device (best effort).
/// This is needed for hybrid ISOs whose backup GPT sits at the end of the image,
/// leaving free space unreachable until the header is moved.
fn maybe_expand_gpt(device: &str) -> UsbCreatorResult<()> {
    match Command::new("sgdisk").args(["-e", device]).output() {
        Ok(output) => {
            if output.status.success() {
                println!("[PERSISTENCE] Expanded GPT to end of device.");
            } else {
                println!(
                    "[PERSISTENCE] Warning: sgdisk -e failed ({}). Continuing.",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(UsbCreatorError::validation_error(
                "sgdisk not found; cannot repair GPT after ISO write. Please install gptfdisk (sgdisk) and retry persistence creation.",
            ));
        }
        Err(e) => {
            return Err(UsbCreatorError::Io(
                e,
                "Failed to run sgdisk -e for GPT expansion".to_string(),
            ));
        }
    }
    Ok(())
}

/// Attempt to fix GPT using parted by auto-answering the prompt to use all space.
fn fix_gpt_with_parted(_device: &str) -> UsbCreatorResult<()> {
    Err(UsbCreatorError::validation_error(
        "GPT appears truncated and sgdisk is missing; install gptfdisk (sgdisk) to repair GPT before adding persistence.",
    ))
}

/// Best-effort udev settle to avoid racing kernel partition table updates
fn settle_udev() {
    if let Ok(output) = Command::new("udevadm").args(["settle"]).output() {
        if !output.status.success() {
            println!(
                "[PERSISTENCE] udevadm settle returned non-zero ({}); continuing.",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }
}

/// Setup Casper persistence (Ubuntu/Debian)
fn setup_casper_persistence(partition_path: &str, _config: &PersistenceConfig) -> UsbCreatorResult<()> {
    let mount_dir = tempfile::tempdir()?;

    // Mount the persistence partition
    run_command("mount", &[partition_path, mount_dir.path().to_str().unwrap()])?;

    let _cleanup = scopeguard::guard((), |_| {
        let _ = run_command("umount", &[partition_path]);
        let _ = run_command("sync", &[]);
    });

    // Create persistence configuration file
    let persistence_conf = mount_dir.path().join("persistence.conf");
    fs::write(&persistence_conf, "/ union\n")?;

    // Create necessary directories for Casper
    let casper_dirs = ["boot", "casper", ".disk"];
    for dir in &casper_dirs {
        fs::create_dir_all(mount_dir.path().join(dir))?;
    }

    // Upper and work directories for overlay
    let overlay_dirs = ["upper", "work"];
    for dir in &overlay_dirs {
        fs::create_dir_all(mount_dir.path().join("casper").join(dir))?;
    }

    Ok(())
}

/// Setup OverlayFS persistence
fn setup_overlayfs_persistence(partition_path: &str, _config: &PersistenceConfig) -> UsbCreatorResult<()> {
    let mount_dir = tempfile::tempdir()?;

    // Mount the persistence partition
    run_command("mount", &[partition_path, mount_dir.path().to_str().unwrap()])?;

    let _cleanup = scopeguard::guard((), |_| {
        let _ = run_command("umount", &[partition_path]);
        let _ = run_command("sync", &[]);
    });

    // Create overlay directories
    let overlay_dirs = ["upper", "work"];
    for dir in &overlay_dirs {
        fs::create_dir_all(mount_dir.path().join(dir))?;
    }

    // Create overlay configuration
    let overlay_conf = mount_dir.path().join("overlay.conf");
    let conf_content =
        "overlayfs\nlowerdir=/rofs\nupperdir=/persistence/upper\nworkdir=/persistence/work\n";
    fs::write(&overlay_conf, conf_content)?;

    Ok(())
}

/// Inject kernel parameters for overlay persistence (Fedora/OverlayFS) if boot configs are writable.
pub fn inject_overlay_kernel_params(usb_device: &str, overlay_label: &str) {
    let candidate_parts = [build_partition_path(usb_device, 1), build_partition_path(usb_device, 2)];
    let candidate_configs = [
        "EFI/BOOT/grub.cfg",
        "EFI/fedora/grub.cfg",
        "EFI/BOOT/grub2.cfg",
        "isolinux/isolinux.cfg",
        "syslinux/isolinux.cfg",
        "syslinux/syslinux.cfg",
        "isolinux.cfg",
    ];
    let param = format!("rd.live.overlay=LABEL={}", overlay_label);

    for part in candidate_parts.iter() {
        let mnt = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => continue,
        };
        if run_command("mount", &[part.as_str(), mnt.path().to_str().unwrap()]).is_err() {
            continue;
        }
        for cfg in candidate_configs.iter() {
            let path = mnt.path().join(cfg);
            if !path.exists() {
                continue;
            }
            if let Ok(contents) = fs::read_to_string(&path) {
                if contents.contains(&param) {
                    continue;
                }
                let mut new_lines = Vec::new();
                for line in contents.lines() {
                    if line.trim_start().starts_with("linux") || line.trim_start().starts_with("linuxefi") {
                        new_lines.push(format!("{} {}", line, param));
                    } else if line.trim_start().starts_with("append") {
                        new_lines.push(format!("{} {}", line, param));
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                if fs::write(&path, new_lines.join("\n")).is_ok() {
                    println!("[PERSISTENCE] Added overlay kernel parameter to {}", path.display());
                }
            }
        }
        let _ = run_command("umount", &[part.as_str()]);
    }
}

/// Setup custom persistence method
fn setup_custom_persistence(
    partition_path: &str,
    config: &PersistenceConfig,
    method: &str,
) -> UsbCreatorResult<()> {
    let mount_dir = tempfile::tempdir()?;

    // Mount the persistence partition
    run_command("mount", &[partition_path, mount_dir.path().to_str().unwrap()])?;

    let _cleanup = scopeguard::guard((), |_| {
        let _ = run_command("umount", &[partition_path]);
        let _ = run_command("sync", &[]);
    });

    // Create custom configuration file
    let custom_conf = mount_dir.path().join("custom-persistence.conf");
    let conf_content = format!("{}\nsize_mb={}\nlabel={}\n", method, config.size_mb, config.label);
    fs::write(&custom_conf, conf_content)?;

    println!("Custom persistence method '{}' configured", method);
    Ok(())
}

/// Detect the appropriate persistence type for a Linux ISO
pub fn detect_persistence_type(iso_path: &str) -> UsbCreatorResult<PersistenceType> {
    let mount_dir = tempfile::tempdir()?;

    // Mount ISO temporarily to check for distribution type
    run_command("mount", &[
        "-o", "loop,ro", iso_path, mount_dir.path().to_str().unwrap()
    ])?;

    let _cleanup = scopeguard::guard((), |_| {
        let _ = run_command("umount", &[mount_dir.path().to_str().unwrap()]);
    });

    // Check for distribution-specific markers
    let mount_path = mount_dir.path();

    // Ubuntu/Debian detection
    if mount_path.join("casper").exists()
        || mount_path.join("disk").join("casper").exists()
        || mount_path.join(".disk").exists()
    {
        return Ok(PersistenceType::Casper);
    }

    // Fedora detection
    if mount_path.join("LiveOS").exists() || mount_path.join("isolinux").exists() {
        return Ok(PersistenceType::OverlayFS);
    }

    // Arch detection
    if mount_path.join("arch").exists() || mount_path.join("airootfs").exists() {
        return Ok(PersistenceType::OverlayFS);
    }

    // Default to OverlayFS for unknown distributions
    Ok(PersistenceType::OverlayFS)
}

/// Validate persistence configuration
pub fn validate_persistence_config(config: &PersistenceConfig) -> UsbCreatorResult<()> {
    if !config.enabled {
        return Ok(());
    }

    if config.size_mb < 512 {
        return Err(UsbCreatorError::validation_error(
            "Persistence size must be at least 512MB",
        ));
    }

    if config.size_mb > 1024 * 32 {
        // 32GB max
        return Err(UsbCreatorError::validation_error(
            "Persistence size cannot exceed 32GB",
        ));
    }

    if config.label.is_empty() {
        return Err(UsbCreatorError::validation_error(
            "Persistence label cannot be empty",
        ));
    }

    Ok(())
}

/// Get recommended persistence size based on ISO size and available space
pub fn get_recommended_persistence_size(
    iso_path: &str,
    device_path: &str,
) -> UsbCreatorResult<u64> {
    // Get ISO size
    let iso_metadata = fs::metadata(iso_path)?;
    let iso_size_mb = iso_metadata.len() / 1024 / 1024;

    // Get device size
    let output = run_command_with_output("lsblk", &["-b", "-nd", "-o", "SIZE", device_path])?;
    let device_size_mb = output.trim().parse::<u64>()? / 1024 / 1024;

    // Calculate available space (reserve 1GB for overhead)
    let available_mb = device_size_mb.saturating_sub(iso_size_mb + 1024);
    if available_mb < 512 {
        return Err(UsbCreatorError::validation_error(
            "Not enough free space for persistence",
        ));
    }

    // Recommend between 2GB and 50% of available space, capped at 32GB
    let half_available = available_mb / 2;
    let recommended_size = half_available.max(2048).min(32 * 1024).min(available_mb);

    Ok(recommended_size)
}

fn run_command(cmd: &str, args: &[&str]) -> UsbCreatorResult<()> {
    println!("[PERSISTENCE] Running command: {} {}", cmd, args.join(" "));
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| UsbCreatorError::Io(e, format!("Failed to spawn {}", cmd)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            println!("[PERSISTENCE] {} stdout: {}", cmd, stdout.trim());
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Treat known non-fatal parted warnings (2048/512) as success
        if cmd == "parted"
            && stderr
                .contains("The driver descriptor says the physical block size is 2048 bytes, but Linux says it is 512 bytes")
        {
            println!("[PERSISTENCE] {} warning about 2048/512 block size; continuing.", cmd);
            return Ok(());
        }
        Err(UsbCreatorError::command_failed(cmd, stderr.trim()))
    }
}

fn run_command_with_output(cmd: &str, args: &[&str]) -> UsbCreatorResult<String> {
    println!("[PERSISTENCE] Running command: {} {}", cmd, args.join(" "));
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| UsbCreatorError::Io(e, format!("Failed to spawn {}", cmd)))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if cmd == "parted"
            && stderr
                .contains("The driver descriptor says the physical block size is 2048 bytes, but Linux says it is 512 bytes")
        {
            println!("[PERSISTENCE] {} warning about 2048/512 block size; continuing.", cmd);
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        Err(UsbCreatorError::command_failed(cmd, stderr.trim()))
    }
}
