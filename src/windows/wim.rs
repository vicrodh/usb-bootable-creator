use std::path::{Path, PathBuf};
use std::process::Command;
use std::io;

pub struct WimEditor {
    wim_path: PathBuf,
}

impl WimEditor {
    pub fn new(wim_path: impl AsRef<Path>) -> Self {
        Self {
            wim_path: wim_path.as_ref().to_path_buf(),
        }
    }

    pub fn has_wimlib() -> bool {
        Command::new("which")
            .arg("wimlib-imagex")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn add_file(
        &self,
        index: u32,
        source_path: &Path,
        wim_target_path: &str,
    ) -> io::Result<()> {
        if !Self::has_wimlib() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "wimlib-imagex not found. Install wimtools/wimlib",
            ));
        }

        // Use wimlib-imagex add with explicit index and target path inside the WIM.
        // Example: wimlib-imagex add boot.wim /tmp/Autounattend.xml /Autounattend.xml --index=2
        let command_str = format!(
            "add \"{}\" \"{}\"",
            source_path.to_string_lossy(),
            wim_target_path
        );

        let status = Command::new("wimlib-imagex")
            .arg("update")
            .arg(&self.wim_path)
            .arg(index.to_string())
            .arg(format!("--command={}", command_str))
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to update WIM at index {}", index),
            ));
        }
        Ok(())
    }

    pub fn verify_index(&self, index: u32) -> io::Result<bool> {
        if !Self::has_wimlib() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "wimlib-imagex not found. Install wimtools/wimlib",
            ));
        }
        let output = Command::new("wimlib-imagex")
            .arg("info")
            .arg(&self.wim_path)
            .arg(index.to_string())
            .output()?;
        Ok(output.status.success())
    }
}
