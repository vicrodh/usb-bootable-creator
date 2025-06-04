use std::process::Command;
use std::io::{self, Write};
use std::fs;
use tempfile::tempdir_in;


/// Write the ISO file to the USB device using dd (requires root)
pub fn write_iso_to_usb(iso_path: &str, usb_device: &str, log: &mut dyn Write) -> io::Result<()> {
    // Use pkexec to run dd as root, not the whole app
    let status = Command::new("pkexec")
        .arg("dd")
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


// Streaming version: print log lines directly to stdout and flush after each
pub fn write_iso_to_usb_stream(iso_path: &str, usb_device: &str) -> io::Result<()> {
    let status = Command::new("pkexec")
        .arg("dd")
        .arg(format!("if={}", iso_path))
        .arg(format!("of={}", usb_device))
        .arg("bs=4M")
        .arg("status=progress")
        .arg("oflag=sync")
        .status()?;

    if status.success() {
        println!("ISO written successfully to {}", usb_device);
        io::stdout().flush().ok();
        Ok(())
    } else {
        println!("Failed to write ISO to {}", usb_device);
        io::stdout().flush().ok();
        Err(io::Error::new(io::ErrorKind::Other, "dd failed"))
    }
}
