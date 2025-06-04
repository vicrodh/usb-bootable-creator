use std::process::Command;
use std::io::{self, Write};

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
