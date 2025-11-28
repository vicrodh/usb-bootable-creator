use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use bitflags::bitflags;

#[derive(Clone, Copy)]
pub enum Architecture {
    X86,
    X64,
    Arm,
    Arm64,
}

impl Architecture {
    fn to_str(self) -> &'static str {
        match self {
            Architecture::X86 => "x86",
            Architecture::X64 => "amd64",
            Architecture::Arm => "arm",
            Architecture::Arm64 => "arm64",
        }
    }
}

bitflags! {
    pub struct UnattendFlags: u32 {
        const BYPASS_TPM = 0x0001;
        const BYPASS_SECURE_BOOT = 0x0002;
        const BYPASS_RAM = 0x0004;
        const BYPASS_ALL = Self::BYPASS_TPM.bits() | Self::BYPASS_SECURE_BOOT.bits() | Self::BYPASS_RAM.bits();
    }
}

pub struct UnattendGenerator {
    arch: Architecture,
    flags: UnattendFlags,
    output_path: PathBuf,
}

impl UnattendGenerator {
    pub fn new(arch: Architecture, flags: UnattendFlags) -> Self {
        Self {
            arch,
            flags,
            output_path: std::env::temp_dir().join("Autounattend.xml"),
        }
    }

    pub fn with_output_path(mut self, path: impl AsRef<Path>) -> Self {
        self.output_path = path.as_ref().to_path_buf();
        self
    }

    pub fn generate(&self) -> io::Result<PathBuf> {
        let mut file = fs::File::create(&self.output_path)?;

        writeln!(file, r#"<?xml version="1.0" encoding="utf-8"?>"#)?;
        writeln!(file, r#"<unattend xmlns="urn:schemas-microsoft-com:unattend">"#)?;

        if self.flags.intersects(UnattendFlags::BYPASS_ALL) {
            self.write_windows_pe_section(&mut file)?;
        }

        writeln!(file, r#"</unattend>"#)?;
        Ok(self.output_path.clone())
    }

    fn write_windows_pe_section(&self, file: &mut fs::File) -> io::Result<()> {
        let arch_name = self.arch.to_str();
        writeln!(file, r#"  <settings pass="windowsPE">"#)?;
        writeln!(
            file,
            r#"    <component name="Microsoft-Windows-Setup" processorArchitecture="{}" language="neutral" publicKeyToken="31bf3856ad364e35" versionScope="nonSxS" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            arch_name
        )?;
        writeln!(file, r#"      <UserData>"#)?;
        writeln!(file, r#"        <ProductKey><Key /></ProductKey>"#)?;
        writeln!(file, r#"      </UserData>"#)?;
        writeln!(file, r#"      <RunSynchronous>"#)?;

        let mut order: u32 = 1;
        if self.flags.contains(UnattendFlags::BYPASS_TPM) {
            self.write_labconfig_command(file, order, "BypassTPMCheck")?;
            order += 1;
        }
        if self.flags.contains(UnattendFlags::BYPASS_SECURE_BOOT) {
            self.write_labconfig_command(file, order, "BypassSecureBootCheck")?;
            order += 1;
        }
        if self.flags.contains(UnattendFlags::BYPASS_RAM) {
            self.write_labconfig_command(file, order, "BypassRAMCheck")?;
        }

        writeln!(file, r#"      </RunSynchronous>"#)?;
        writeln!(file, r#"    </component>"#)?;
        writeln!(file, r#"  </settings>"#)?;
        Ok(())
    }

    fn write_labconfig_command(
        &self,
        file: &mut fs::File,
        order: u32,
        key_name: &str,
    ) -> io::Result<()> {
        writeln!(file, r#"        <RunSynchronousCommand wcm:action="add">"#)?;
        writeln!(file, r#"          <Order>{}</Order>"#, order)?;
        writeln!(
            file,
            r#"          <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v {} /t REG_DWORD /d 1 /f</Path>"#,
            key_name
        )?;
        writeln!(file, r#"        </RunSynchronousCommand>"#)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unattend_with_labconfig_entries() {
        let generator = UnattendGenerator::new(Architecture::X64, UnattendFlags::BYPASS_ALL)
            .with_output_path(std::env::temp_dir().join("autounattend_test.xml"));
        let path = generator.generate().expect("failed to generate unattend");
        let content = fs::read_to_string(path).expect("failed to read unattend");
        assert!(content.contains("BypassTPMCheck"));
        assert!(content.contains("BypassSecureBootCheck"));
        assert!(content.contains("BypassRAMCheck"));
        assert!(content.contains("windowsPE"));
    }
}
