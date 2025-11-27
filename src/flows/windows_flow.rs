use std::fs;
use std::process::Command;
use std::io::{self, BufRead, Write};
use std::time::Instant;

use crate::utils::{get_device_optimal_block_size, has_ntfs3g, is_usb_device, parse_rsync_progress};
use tempfile::tempdir_in;

/// Metrics captured during the Windows USB creation flow.
#[derive(Debug, Default, Clone)]
pub struct WindowsFlowMetrics {
    pub partition_time_ms: u64,
    pub format_time_ms: u64,
    pub boot_copy_time_ms: u64,
    pub install_copy_time_ms: u64,
    pub total_bytes: u64,
    pub avg_speed_mbps: f64,
    pub peak_speed_mbps: f64,
}

fn log_metrics(metrics: &WindowsFlowMetrics, log: &mut dyn Write) -> io::Result<()> {
    writeln!(log, "---- Windows USB creation metrics ----")?;
    writeln!(log, "Partitioning time  : {} ms", metrics.partition_time_ms)?;
    writeln!(log, "Formatting time    : {} ms", metrics.format_time_ms)?;
    writeln!(log, "BOOT copy time     : {} ms", metrics.boot_copy_time_ms)?;
    writeln!(log, "INSTALL copy time  : {} ms", metrics.install_copy_time_ms)?;
    writeln!(log, "Total bytes copied : {} bytes", metrics.total_bytes)?;
    writeln!(log, "Average speed      : {:.2} MB/s", metrics.avg_speed_mbps)?;
    writeln!(log, "Peak speed         : {:.2} MB/s", metrics.peak_speed_mbps)?;
    writeln!(log, "--------------------------------------")?;
    Ok(())
}

fn run_rsync_with_metrics(
    args: &[String],
    peak_speed: &mut f64,
) -> io::Result<u64> {
    let mut command = Command::new("rsync");
    command.args(args);
    command.stdout(std::process::Stdio::null());
    command.stderr(std::process::Stdio::piped());

    let mut child = command.spawn()?;
    let mut transferred: u64 = 0;

    if let Some(stderr) = child.stderr.take() {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Some((bytes, speed_mbps_opt)) = parse_rsync_progress(&line) {
                    transferred = transferred.max(bytes);
                    if let Some(speed) = speed_mbps_opt {
                        if speed > *peak_speed {
                            *peak_speed = speed;
                        }
                    }
                }
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "rsync failed"));
    }

    Ok(transferred)
}

fn ensure_not_system_device(device: &str, log: &mut dyn Write) -> io::Result<()> {
    let dev_base = device.trim_start_matches("/dev/");
    let output = Command::new("lsblk")
        .args(["-nr", "-o", "NAME,MOUNTPOINT"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(mountpoint)) = (parts.next(), parts.next()) {
            if name.starts_with(dev_base) && (mountpoint == "/" || mountpoint == "/boot" || mountpoint == "/boot/efi") {
                writeln!(
                    log,
                    "Refusing to operate on system device {} (mounted at {})",
                    device, mountpoint
                )?;
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Device {} appears to host system mount {}", device, mountpoint),
                ));
            }
        }
    }
    Ok(())
}

fn unmount_device_mounts(device: &str, log: &mut dyn Write) -> io::Result<()> {
    let dev_name = device.trim_start_matches("/dev/");
    let output = Command::new("lsblk")
        .args(["-nr", "-o", "NAME,MOUNTPOINT"])
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    for line in output_str.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(mountpoint)) = (parts.next(), parts.next()) {
            if name.starts_with(dev_name) && !mountpoint.is_empty() && mountpoint != "/" && mountpoint != "/boot" && mountpoint != "/boot/efi" {
                writeln!(log, "Unmounting busy mount {}...", mountpoint)?;
                let status = Command::new("umount").args(["-f", mountpoint]).status()?;
                if !status.success() {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to unmount {}", mountpoint),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn write_windows_iso_to_usb(iso_path: &str, usb_device: &str, use_wim: bool, log: &mut dyn Write) -> io::Result<WindowsFlowMetrics> {
    let _ = use_wim; // Placeholder to maintain signature parity until WIM handling is implemented.
    let overall_start = Instant::now();
    let mut metrics = WindowsFlowMetrics::default();
    let mut peak_speed_mbps = 0.0;

    // Create temp mount dirs under /mnt
    let base = tempdir_in("/mnt").map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create tempdir: {}", e)))?;
    let iso_m = base.path().join("iso");
    let boot_m = base.path().join("boot");
    let inst_m = base.path().join("install");
    for m in [&iso_m, &boot_m, &inst_m] {
        fs::create_dir_all(m)?;
    }
    // Safety: refuse to operate on system devices and unmount removable mounts.
    ensure_not_system_device(usb_device, log)?;
    // Ensure device and its partitions are unmounted before wipefs/partitioning.
    unmount_device_mounts(usb_device, log)?;
    let mut cleanup = || {
        let _ = Command::new("umount").arg(&inst_m).status();
        let _ = Command::new("umount").arg(&boot_m).status();
        let _ = Command::new("umount").arg(&iso_m).status();
        let _ = fs::remove_dir_all(base.path());
        let _ = Command::new("sync").status();
    };
    // Stage 1: wipe and partition
    let partition_start = Instant::now();
    writeln!(log, "Wiping and partitioning...")?;
    let status = Command::new("wipefs").arg("-a").arg(usb_device).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "wipefs failed")); }
    let status = Command::new("parted").args(["-s", usb_device, "mklabel", "gpt"]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "parted mklabel failed")); }
    // Create partitions
    let parts = [
        ("BOOT", "fat32", "1GiB", "BOOT"),
        ("ESD-USB", "ntfs", "100%", "ESD-USB")
    ];
    let mut start = "0%";
    for (label, fstype, end, _vol) in parts.iter() {
        writeln!(log, "Creating partition {}...", label)?;
        let status = Command::new("parted").args(["-s", usb_device, "mkpart", label, fstype, start, end]).status()?;
        if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "parted mkpart failed")); }
        start = end;
    }
    metrics.partition_time_ms = partition_start.elapsed().as_millis() as u64;
    // Format partitions
    let format_start = Instant::now();
    let p1 = format!("{}1", usb_device);
    let p2 = format!("{}2", usb_device);
    writeln!(log, "Formatting BOOT as FAT32...")?;
    let block_size = match get_device_optimal_block_size(usb_device) {
        Ok(size) => {
            writeln!(log, "Detected optimal block size: {} bytes", size)?;
            size
        }
        Err(e) => {
            writeln!(log, "Warning: could not detect block size ({}), falling back to 4096", e)?;
            4096
        }
    };
    let sectors_per_cluster = ((block_size / 512).max(1)).min(64); // FAT32 sectors per cluster
    let fat_cluster_bytes = sectors_per_cluster * 512;
    writeln!(log, "Using FAT32 cluster size: {} bytes ({} sectors)", fat_cluster_bytes, sectors_per_cluster)?;

    let status = Command::new("mkfs.vfat")
        .args([
            "-F32",
            "-s",
            &sectors_per_cluster.to_string(),
            "-n",
            "BOOT",
            &p1,
        ])
        .status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.vfat failed")); }
    writeln!(log, "Formatting INSTALL as NTFS...")?;
    let ntfs_cluster = block_size.clamp(512, 65536);
    let status = Command::new("mkfs.ntfs")
        .args([
            "--quick",
            "-c",
            &ntfs_cluster.to_string(),
            "-L",
            "ESD-USB",
            &p2,
        ])
        .status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.ntfs failed")); }
    metrics.format_time_ms = format_start.elapsed().as_millis() as u64;
    // Mount ISO
    writeln!(log, "Mounting ISO...")?;
    let status = Command::new("mount").args(["-o", "loop,ro", iso_path, iso_m.to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount ISO failed")); }
    // Copy BOOT files
    writeln!(log, "Mounting BOOT partition...")?;
    let status = Command::new("mount").args([&p1, boot_m.to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount BOOT failed")); }
    writeln!(log, "Copying files to BOOT...")?;
    let boot_copy_start = Instant::now();
    let mut boot_args = vec![
        "-a".to_string(),
        "--no-owner".to_string(),
        "--no-group".to_string(),
        "--no-inc-recursive".to_string(),
        "--inplace".to_string(),
        "--info=progress2".to_string(),
        "--exclude".to_string(),
        "sources/".to_string(),
        format!("{}/", iso_m.to_str().unwrap()),
        format!("{}/", boot_m.to_str().unwrap()),
    ];
    if is_usb_device(usb_device) {
        boot_args.push("--whole-file".to_string());
    }
    let boot_transferred = run_rsync_with_metrics(&boot_args, &mut peak_speed_mbps).map_err(|e| {
        cleanup();
        io::Error::new(io::ErrorKind::Other, format!("rsync BOOT failed: {}", e))
    })?;
    metrics.boot_copy_time_ms = boot_copy_start.elapsed().as_millis() as u64;
    metrics.total_bytes = metrics.total_bytes.saturating_add(boot_transferred);

    writeln!(log, "Copying boot.wim...")?;
    let _ = fs::create_dir_all(boot_m.join("sources"));
    let status = Command::new("cp").args([iso_m.join("sources/boot.wim").to_str().unwrap(), boot_m.join("sources").to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "cp boot.wim failed")); }
    // Copy INSTALL files
    writeln!(log, "Mounting INSTALL partition...")?;
    let ntfs_opts = if has_ntfs3g() {
        "big_writes,async,noatime,nodiratime"
    } else {
        "noatime,nodiratime"
    };
    let status = if has_ntfs3g() {
        Command::new("mount")
            .args(["-t", "ntfs-3g", "-o", ntfs_opts, &p2, inst_m.to_str().unwrap()])
            .status()
    } else {
        Command::new("mount")
            .args(["-o", ntfs_opts, &p2, inst_m.to_str().unwrap()])
            .status()
    }?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount INSTALL failed")); }
    writeln!(log, "Copying files to INSTALL...")?;
    let install_copy_start = Instant::now();
    let mut install_args = vec![
        "-a".to_string(),
        "--no-owner".to_string(),
        "--no-group".to_string(),
        "--no-inc-recursive".to_string(),
        "--inplace".to_string(),
        "--info=progress2".to_string(),
        format!("{}/", iso_m.to_str().unwrap()),
        format!("{}/", inst_m.to_str().unwrap()),
    ];
    if is_usb_device(usb_device) {
        install_args.push("--whole-file".to_string());
    }
    let install_transferred = run_rsync_with_metrics(&install_args, &mut peak_speed_mbps).map_err(|e| {
        cleanup();
        io::Error::new(io::ErrorKind::Other, format!("rsync INSTALL failed: {}", e))
    })?;
    metrics.install_copy_time_ms = install_copy_start.elapsed().as_millis() as u64;
    metrics.total_bytes = metrics.total_bytes.saturating_add(install_transferred);

    // Cleanup
    writeln!(log, "Cleaning up mounts...")?;
    cleanup();
    let total_secs = overall_start.elapsed().as_secs_f64().max(f64::EPSILON);
    metrics.avg_speed_mbps = (metrics.total_bytes as f64 / total_secs) / 1_000_000.0;
    metrics.peak_speed_mbps = peak_speed_mbps;

    log_metrics(&metrics, log)?;
    writeln!(log, "Windows USB creation completed.")?;
    Ok(metrics)
}

// Helper for verbose step output
fn print_step(step: usize, total: usize, msg: &str) {
    println!("[STEP] {}/{}: {}", step, total, msg);
    std::io::stdout().flush().ok();
}
fn print_error(step: usize, total: usize, msg: &str) {
    println!("[ERROR] {}/{}: {}", step, total, msg);
    std::io::stdout().flush().ok();
}

// Streaming version: print log lines directly to stdout and flush after each
pub fn write_windows_iso_to_usb_stream(iso_path: &str, usb_device: &str, cluster_bytes: u64) -> io::Result<()> {
    let total_steps = 15;
    let mut step = 1;
    let _ = cluster_bytes; // preserved for signature compatibility
    let base = tempfile::tempdir_in("/mnt").map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create tempdir: {}", e)))?;
    let iso_m = base.path().join("iso");
    let boot_m = base.path().join("boot");
    let inst_m = base.path().join("install");
    for m in [&iso_m, &boot_m, &inst_m] {
        std::fs::create_dir_all(m)?;
    }
    // Safety: refuse to operate on system devices.
    ensure_not_system_device(usb_device, &mut std::io::sink())?;
    // Ensure device and its partitions are unmounted before wipefs/partitioning.
    {
        let dev_name = usb_device.trim_start_matches("/dev/");
        if let Ok(output) = std::process::Command::new("lsblk").args(["-nr", "-o", "NAME,MOUNTPOINT"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            for line in out.lines() {
                let mut parts = line.split_whitespace();
                if let (Some(name), Some(mountpoint)) = (parts.next(), parts.next()) {
                    if name.starts_with(dev_name) && !mountpoint.is_empty() {
                        println!("Unmounting busy mount {}...", mountpoint);
                        let _ = std::process::Command::new("umount").args(["-f", mountpoint]).status();
                    }
                }
            }
        }
    }
    let cleanup = || {
        let _ = std::process::Command::new("umount").arg(&inst_m).status();
        let _ = std::process::Command::new("umount").arg(&boot_m).status();
        let _ = std::process::Command::new("umount").arg(&iso_m).status();
        let _ = std::fs::remove_dir_all(base.path());
        let _ = std::process::Command::new("sync").status();
    };
    print_step(step, total_steps, "Wiping and partitioning..."); step += 1;
    let status = std::process::Command::new("wipefs").arg("-a").arg(usb_device).status()?;
    if !status.success() { print_error(step, total_steps, "wipefs failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "wipefs failed")); }
    print_step(step, total_steps, "Creating GPT partition table..."); step += 1;
    let status = std::process::Command::new("parted").args(["-s", usb_device, "mklabel", "gpt"]).status()?;
    if !status.success() { print_error(step, total_steps, "parted mklabel failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "parted mklabel failed")); }
    let parts = [
        ("BOOT", "fat32", "1GiB", "BOOT"),
        ("ESD-USB", "ntfs", "100%", "ESD-USB")
    ];
    let mut start = "0%";
    for (label, fstype, end, _vol) in parts.iter() {
        print_step(step, total_steps, &format!("Creating partition {}...", label)); step += 1;
        let status = std::process::Command::new("parted").args(["-s", usb_device, "mkpart", label, fstype, start, end]).status()?;
        if !status.success() { print_error(step, total_steps, &format!("parted mkpart {} failed", label)); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "parted mkpart failed")); }
        start = end;
    }
    let p1 = format!("{}1", usb_device);
    let p2 = format!("{}2", usb_device);
    let block_size = match get_device_optimal_block_size(usb_device) {
        Ok(size) => {
            println!("Detected optimal block size: {} bytes", size);
            size
        }
        Err(e) => {
            println!("Warning: could not detect block size ({}), falling back to 4096", e);
            4096
        }
    };
    let sectors_per_cluster = ((block_size / 512).max(1)).min(64); // FAT32 sectors per cluster
    let fat_cluster_bytes = sectors_per_cluster * 512;
    println!("Using FAT32 cluster size: {} bytes ({} sectors)", fat_cluster_bytes, sectors_per_cluster);

    print_step(step, total_steps, "Formatting BOOT as FAT32..."); step += 1;
    let status = std::process::Command::new("mkfs.vfat")
        .args(["-F32", "-s", &sectors_per_cluster.to_string(), "-n", "BOOT", &p1])
        .status()?;
    if !status.success() { print_error(step, total_steps, "mkfs.vfat failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.vfat failed")); }
    print_step(step, total_steps, "Formatting INSTALL as NTFS..."); step += 1;
    let ntfs_cluster = block_size.clamp(512, 65536);
    let status = std::process::Command::new("mkfs.ntfs")
        .args(["--quick", "-c", &ntfs_cluster.to_string(), "-L", "ESD-USB", &p2])
        .status()?;
    if !status.success() { print_error(step, total_steps, "mkfs.ntfs failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.ntfs failed")); }
    print_step(step, total_steps, "Mounting ISO..."); step += 1;
    let status = std::process::Command::new("mount").args(["-o", "loop,ro", iso_path, iso_m.to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "mount ISO failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount ISO failed")); }
    print_step(step, total_steps, "Mounting BOOT partition..."); step += 1;
    let status = std::process::Command::new("mount").args([&p1, boot_m.to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "mount BOOT failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount BOOT failed")); }
    print_step(step, total_steps, "Copying files to BOOT..."); step += 1;
    let mut boot_args = vec![
        "-a".to_string(),
        "--no-owner".to_string(),
        "--no-group".to_string(),
        "--no-inc-recursive".to_string(),
        "--inplace".to_string(),
        "--info=progress2".to_string(),
        "--exclude".to_string(),
        "sources/".to_string(),
        format!("{}/", iso_m.to_str().unwrap()),
        format!("{}/", boot_m.to_str().unwrap()),
    ];
    if is_usb_device(usb_device) {
        boot_args.push("--whole-file".to_string());
    }
    let status = std::process::Command::new("rsync").args(boot_args).status()?;
    if !status.success() { print_error(step, total_steps, "rsync BOOT failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "rsync BOOT failed")); }
    print_step(step, total_steps, "Copying boot.wim..."); step += 1;
    let _ = std::fs::create_dir_all(boot_m.join("sources"));
    let status = std::process::Command::new("cp").args([iso_m.join("sources/boot.wim").to_str().unwrap(), boot_m.join("sources").to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "cp boot.wim failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "cp boot.wim failed")); }
    print_step(step, total_steps, "Mounting INSTALL partition..."); step += 1;
    let ntfs_opts = if has_ntfs3g() {
        "big_writes,async,noatime,nodiratime"
    } else {
        "noatime,nodiratime"
    };
    let status = if has_ntfs3g() {
        std::process::Command::new("mount")
            .args(["-t", "ntfs-3g", "-o", ntfs_opts, &p2, inst_m.to_str().unwrap()])
            .status()
    } else {
        std::process::Command::new("mount")
            .args(["-o", ntfs_opts, &p2, inst_m.to_str().unwrap()])
            .status()
    }?;
    if !status.success() { print_error(step, total_steps, "mount INSTALL failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount INSTALL failed")); }
    print_step(step, total_steps, "Copying files to INSTALL; Please wait this could take a bit..."); step += 1;
    let mut install_args = vec![
        "-a".to_string(),
        "--no-owner".to_string(),
        "--no-group".to_string(),
        "--no-inc-recursive".to_string(),
        "--inplace".to_string(),
        "--info=progress2".to_string(),
        format!("{}/", iso_m.to_str().unwrap()),
        format!("{}/", inst_m.to_str().unwrap()),
    ];
    if is_usb_device(usb_device) {
        install_args.push("--whole-file".to_string());
    }
    let status = std::process::Command::new("rsync").args(install_args).status()?;
    if !status.success() { print_error(step, total_steps, "rsync INSTALL failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "rsync INSTALL failed")); }
    print_step(step, total_steps, "Cleaning up mounts; We're almost done, please wait..."); step += 1;
    cleanup();
    print_step(step, total_steps, "Windows USB creation completed.");
    Ok(())
}
