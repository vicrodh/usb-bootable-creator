//! Custom error types for the USB bootable creator

use std::fmt;
use std::io;

/// Custom error types for USB creation operations
#[derive(Debug)]
pub enum UsbCreatorError {
    /// IO-related errors with context
    Io(io::Error, String),

    /// Command execution errors
    CommandFailed(String, String),

    /// Partition-related errors
    PartitionError(String),

    /// Mount-related errors
    MountError(String),

    /// ISO detection errors
    IsoDetectionError(String),

    /// Package dependency errors
    PackageError(String),

    /// Configuration errors
    ConfigError(String),

    /// Permission/privilege errors
    PermissionError(String),

    /// Validation errors
    ValidationError(String),

    /// Generic errors with context
    Generic(String),
}

impl fmt::Display for UsbCreatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbCreatorError::Io(err, context) => write!(f, "IO Error: {} - {}", context, err),
            UsbCreatorError::CommandFailed(cmd, output) => {
                write!(f, "Command failed: {} - {}", cmd, output)
            }
            UsbCreatorError::PartitionError(msg) => write!(f, "Partition error: {}", msg),
            UsbCreatorError::MountError(msg) => write!(f, "Mount error: {}", msg),
            UsbCreatorError::IsoDetectionError(msg) => write!(f, "ISO detection error: {}", msg),
            UsbCreatorError::PackageError(msg) => write!(f, "Package error: {}", msg),
            UsbCreatorError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            UsbCreatorError::PermissionError(msg) => write!(f, "Permission error: {}", msg),
            UsbCreatorError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            UsbCreatorError::Generic(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for UsbCreatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UsbCreatorError::Io(err, _) => Some(err),
            _ => None,
        }
    }
}

/// Result type alias for convenience
pub type UsbCreatorResult<T> = Result<T, UsbCreatorError>;

/// Helper trait for converting IO errors with context
pub trait IoResultExt<T> {
    fn with_context(self, context: impl Into<String>) -> UsbCreatorResult<T>;
}

impl<T> IoResultExt<T> for Result<T, io::Error> {
    fn with_context(self, context: impl Into<String>) -> UsbCreatorResult<T> {
        self.map_err(|e| UsbCreatorError::Io(e, context.into()))
    }
}

/// Error conversion helpers
impl From<io::Error> for UsbCreatorError {
    fn from(err: io::Error) -> Self {
        UsbCreatorError::Io(err, "IO operation failed".to_string())
    }
}

impl From<anyhow::Error> for UsbCreatorError {
    fn from(err: anyhow::Error) -> Self {
        UsbCreatorError::Generic(err.to_string())
    }
}

impl From<std::num::ParseIntError> for UsbCreatorError {
    fn from(err: std::num::ParseIntError) -> Self {
        UsbCreatorError::ValidationError(format!("Failed to parse number: {}", err))
    }
}

/// Error creation helpers
impl UsbCreatorError {
    pub fn command_failed(command: &str, output: &str) -> Self {
        UsbCreatorError::CommandFailed(command.to_string(), output.to_string())
    }

    pub fn partition_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::PartitionError(msg.into())
    }

    pub fn mount_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::MountError(msg.into())
    }

    pub fn iso_detection_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::IsoDetectionError(msg.into())
    }

    pub fn package_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::PackageError(msg.into())
    }

    pub fn config_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::ConfigError(msg.into())
    }

    pub fn permission_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::PermissionError(msg.into())
    }

    pub fn validation_error(msg: impl Into<String>) -> Self {
        UsbCreatorError::ValidationError(msg.into())
    }

    pub fn generic(msg: impl Into<String>) -> Self {
        UsbCreatorError::Generic(msg.into())
    }
}