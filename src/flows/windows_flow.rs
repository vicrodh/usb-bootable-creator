use std::process::Command;
use std::io::{self, Write};
use std::fs;
use tempfile::tempdir_in;

pub fn write_windows_iso_to_usb(iso_path: &str, usb_device: &str, use_wim: bool, log: &mut dyn Write) -> io::Result<()> {
    // Create temp mount dirs under /mnt
    let base = tempdir_in("/mnt").map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create tempdir: {}", e)))?;
    let iso_m = base.path().join("iso");
    let boot_m = base.path().join("boot");
    let inst_m = base.path().join("install");
    for m in [&iso_m, &boot_m, &inst_m] {
        fs::create_dir_all(m)?;
    }
    let mut cleanup = || {
        let _ = Command::new("umount").arg(&inst_m).status();
        let _ = Command::new("umount").arg(&boot_m).status();
        let _ = Command::new("umount").arg(&iso_m).status();
        let _ = fs::remove_dir_all(base.path());
        let _ = Command::new("sync").status();
    };
    // Stage 1: wipe and partition
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
    // Format partitions
    let p1 = format!("{}1", usb_device);
    let p2 = format!("{}2", usb_device);
    writeln!(log, "Formatting BOOT as FAT32...")?;
    let status = Command::new("mkfs.vfat").args(["-F32", "-n", "BOOT", &p1]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.vfat failed")); }
    writeln!(log, "Formatting INSTALL as NTFS...")?;
    let status = Command::new("mkfs.ntfs").args(["--quick", "-L", "ESD-USB", &p2]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.ntfs failed")); }
    // Mount ISO
    writeln!(log, "Mounting ISO...")?;
    let status = Command::new("mount").args(["-o", "loop,ro", iso_path, iso_m.to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount ISO failed")); }
    // Copy BOOT files
    writeln!(log, "Mounting BOOT partition...")?;
    let status = Command::new("mount").args([&p1, boot_m.to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount BOOT failed")); }
    writeln!(log, "Copying files to BOOT...")?;
    let status = Command::new("rsync").args(["-a", "--no-owner", "--no-group", "--exclude", "sources/", &format!("{}/", iso_m.to_str().unwrap()), &format!("{}/", boot_m.to_str().unwrap())]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "rsync BOOT failed")); }
    writeln!(log, "Copying boot.wim...")?;
    let _ = fs::create_dir_all(boot_m.join("sources"));
    let status = Command::new("cp").args([iso_m.join("sources/boot.wim").to_str().unwrap(), boot_m.join("sources").to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "cp boot.wim failed")); }
    // Copy INSTALL files
    writeln!(log, "Mounting INSTALL partition...")?;
    let status = Command::new("mount").args([&p2, inst_m.to_str().unwrap()]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount INSTALL failed")); }
    writeln!(log, "Copying files to INSTALL...")?;
    let status = Command::new("rsync").args(["-a", "--no-owner", "--no-group", &format!("{}/", iso_m.to_str().unwrap()), &format!("{}/", inst_m.to_str().unwrap())]).status()?;
    if !status.success() { cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "rsync INSTALL failed")); }
    // Cleanup
    writeln!(log, "Cleaning up mounts...")?;
    cleanup();
    writeln!(log, "Windows USB creation completed.")?;
    Ok(())
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
    let base = tempfile::tempdir_in("/mnt").map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create tempdir: {}", e)))?;
    let iso_m = base.path().join("iso");
    let boot_m = base.path().join("boot");
    let inst_m = base.path().join("install");
    for m in [&iso_m, &boot_m, &inst_m] {
        std::fs::create_dir_all(m)?;
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
    print_step(step, total_steps, "Formatting BOOT as FAT32..."); step += 1;
    let status = std::process::Command::new("mkfs.vfat").args(["-F32", "-n", "BOOT", &p1]).status()?;
    if !status.success() { print_error(step, total_steps, "mkfs.vfat failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.vfat failed")); }
    print_step(step, total_steps, "Formatting INSTALL as NTFS..."); step += 1;
    let status = std::process::Command::new("mkfs.ntfs").args(["--quick", "-c", &cluster_bytes.to_string(), "-L", "ESD-USB", &p2]).status()?;
    if !status.success() { print_error(step, total_steps, "mkfs.ntfs failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mkfs.ntfs failed")); }
    print_step(step, total_steps, "Mounting ISO..."); step += 1;
    let status = std::process::Command::new("mount").args(["-o", "loop,ro", iso_path, iso_m.to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "mount ISO failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount ISO failed")); }
    print_step(step, total_steps, "Mounting BOOT partition..."); step += 1;
    let status = std::process::Command::new("mount").args([&p1, boot_m.to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "mount BOOT failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount BOOT failed")); }
    print_step(step, total_steps, "Copying files to BOOT..."); step += 1;
    let status = std::process::Command::new("rsync").args(["-a", "--no-owner", "--no-group", "--exclude", "sources/", &format!("{}/", iso_m.to_str().unwrap()), &format!("{}/", boot_m.to_str().unwrap())]).status()?;
    if !status.success() { print_error(step, total_steps, "rsync BOOT failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "rsync BOOT failed")); }
    print_step(step, total_steps, "Copying boot.wim..."); step += 1;
    let _ = std::fs::create_dir_all(boot_m.join("sources"));
    let status = std::process::Command::new("cp").args([iso_m.join("sources/boot.wim").to_str().unwrap(), boot_m.join("sources").to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "cp boot.wim failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "cp boot.wim failed")); }
    print_step(step, total_steps, "Mounting INSTALL partition..."); step += 1;
    let status = std::process::Command::new("mount").args([&p2, inst_m.to_str().unwrap()]).status()?;
    if !status.success() { print_error(step, total_steps, "mount INSTALL failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "mount INSTALL failed")); }
    print_step(step, total_steps, "Copying files to INSTALL; Please wait this could take a bit..."); step += 1;
    let status = std::process::Command::new("rsync").args(["-a", "--no-owner", "--no-group", &format!("{}/", iso_m.to_str().unwrap()), &format!("{}/", inst_m.to_str().unwrap())]).status()?;
    if !status.success() { print_error(step, total_steps, "rsync INSTALL failed"); cleanup(); return Err(io::Error::new(io::ErrorKind::Other, "rsync INSTALL failed")); }
    print_step(step, total_steps, "Cleaning up mounts; We're almost done, please wait..."); step += 1;
    cleanup();
    print_step(step, total_steps, "Windows USB creation completed.");
    Ok(())
}
