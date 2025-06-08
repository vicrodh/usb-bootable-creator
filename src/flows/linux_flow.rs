use std::process::Command;
use std::io::{self, Write};
use std::fs;
use tempfile::tempdir_in;


/// Write the ISO file to the USB device using dd (requires root)
pub fn write_iso_to_usb(iso_path: &str, usb_device: &str, log: &mut dyn Write) -> io::Result<()> {
    let status = Command::new("dd")
        .arg(format!("if={}", iso_path))
        .arg(format!("of={}", usb_device))
        .arg("bs=4M")
        .arg("status=progress")
        .arg("oflag=sync")
        .status()?;

    if status.success() {
        writeln!(log, "ISO written successfully to {}", usb_device)?;
        Ok(())
    } else {
        writeln!(log, "Failed to write ISO to {}", usb_device)?;
        Err(io::Error::new(io::ErrorKind::Other, "dd failed"))
    }
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


/// Streaming version: print log lines directly to stdout and flush after each
pub fn write_iso_to_usb_stream(iso_path: &str, usb_device: &str, cluster_bytes: u64) -> io::Result<()> {
    let total_steps = 5;
    let mut step = 1;
    print_step(step, total_steps, "Wiping old partition table (wipefs)...");
    let status = Command::new("wipefs")
        .arg("-a")
        .arg(usb_device)
        .status()?;
    if !status.success() {
        print_error(step, total_steps, "Failed to wipe partition table");
        return Err(io::Error::new(io::ErrorKind::Other, "wipefs failed"));
    }
    step += 1;
    // Pre-fetch ISO size
    let iso_size = std::fs::metadata(iso_path).map(|m| m.len()).unwrap_or(0);
    print_step(step, total_steps, &format!("Writing ISO to USB with dd (this may take a while)..."));
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader, Write};
    let mut child = Command::new("dd")
        .arg(format!("if={}", iso_path))
        .arg(format!("of={}", usb_device))
        .arg(format!("bs={}", cluster_bytes))
        .arg("status=progress")
        .arg("oflag=sync")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stderr = child.stderr.take().unwrap();
    let mut reader = BufReader::new(stderr);
    let mut buf = String::new();
    let mut last_percent = 0;
    let mut last_mb = 0;
    while let Ok(bytes) = reader.read_line(&mut buf) {
        if bytes == 0 { break; }
        if let Some(num) = buf.trim().split_whitespace().next() {
            if let Ok(bytes_copied) = num.parse::<u64>() {
                if iso_size > 0 {
                    let percent = ((bytes_copied as f64 / iso_size as f64) * 100.0) as u8;
                    let mb_copied = bytes_copied / 1024 / 1024;
                    // Always print the updatable step line with current MB copied
                    print_step(step, total_steps, &format!(
                        "Writing ISO to USB with dd (this may take a while)..."
                    ));
                    if percent != last_percent && percent % 5 == 0 {
                        println!("[PROGRESS] dd: {} MB / {:.1} MB ({}%)", mb_copied, iso_size as f64 / 1024.0 / 1024.0, percent);
                        std::io::stdout().flush().ok();
                        last_percent = percent;
                        last_mb = mb_copied;
                    }
                } else {
                    let mb_copied = bytes_copied / 1024 / 1024;
                    if mb_copied > last_mb {
                        println!("[PROGRESS] dd: {} MB written", mb_copied);
                        std::io::stdout().flush().ok();
                        last_mb = mb_copied;
                    }
                }
            }
        }
        buf.clear();
    }
    let status = child.wait()?;
    if !status.success() {
        print_error(step, total_steps, "Failed to write ISO to USB");
        return Err(io::Error::new(io::ErrorKind::Other, "dd failed"));
    }
    step += 1;
    print_step(step, total_steps, "Syncing data to disk...");
    let _ = Command::new("sync").status();
    step += 1;
    print_step(step, total_steps, "Finalizing...");
    step += 1;
    print_step(step, total_steps, "Linux USB creation completed.");
    Ok(())
}
