//! Configuration constants for the USB bootable creator

/// Windows partition configuration
pub mod windows {
    /// BOOT partition size in GB
    pub const BOOT_PARTITION_SIZE_GB: &str = "1GiB";

    /// BOOT partition label
    pub const BOOT_PARTITION_LABEL: &str = "BOOT";

    /// ESD-USB partition label
    pub const ESD_PARTITION_LABEL: &str = "ESD-USB";

    /// BOOT partition filesystem
    pub const BOOT_FILESYSTEM: &str = "fat32";

    /// ESD-USB partition filesystem
    pub const ESD_FILESYSTEM: &str = "ntfs";
}

/// NTFS cluster size options (in bytes)
pub const NTFS_CLUSTER_SIZES: &[(u64, &str)] = &[
    (512, "512 bytes"),
    (1024, "1K"),
    (2048, "2K"),
    (4096, "4K"),
    (8192, "8K"),
    (16384, "16K"),
    (32768, "32K"),
    (65536, "64K"),
];

/// Default NTFS cluster size index in NTFS_CLUSTER_SIZES
pub const DEFAULT_CLUSTER_SIZE_INDEX: usize = 3; // 4K

/// Linux ISO writing configuration
pub mod linux {
    /// dd block size for ISO writing
    pub const DD_BLOCK_SIZE: &str = "4M";

    /// dd block size in bytes (for fallback)
    pub const DD_BLOCK_SIZE_BYTES: u64 = 4 * 1024 * 1024;
}

/// Progress reporting configuration
pub mod progress {
    /// Total steps for Windows ISO creation
    pub const WINDOWS_TOTAL_STEPS: usize = 15;

    /// Total steps for Linux ISO creation
    pub const LINUX_TOTAL_STEPS: usize = 5;

    /// Progress reporting interval (percentage)
    pub const PROGRESS_REPORT_INTERVAL: u8 = 5;

    /// Progress display minimum MB increment
    pub const PROGRESS_MB_INCREMENT: u64 = 100;
}

/// Temporary directory configuration
pub mod temp {
    /// Base directory for temporary mounts
    pub const MOUNT_BASE: &str = "/mnt";

    /// ISO mount directory name
    pub const ISO_MOUNT_DIR: &str = "iso";

    /// Boot partition mount directory name
    pub const BOOT_MOUNT_DIR: &str = "boot";

    /// Install partition mount directory name
    pub const INSTALL_MOUNT_DIR: &str = "install";
}

/// GUI configuration
pub mod gui {
    /// Application ID
    pub const APP_ID: &str = "com.example.usbbootablecreator";

    /// Window title
    pub const WINDOW_TITLE: &str = "MajUSB Bootable Creator";

    /// Default window dimensions
    pub const DEFAULT_WIDTH: i32 = 830;
    pub const DEFAULT_HEIGHT: i32 = 400;

    /// Minimum window dimensions
    pub const MIN_WIDTH: i32 = 770;
    pub const MIN_HEIGHT: i32 = 400;

    /// Widget spacing
    pub const WIDGET_SPACING: i32 = 8;
    pub const SECTION_SPACING: i32 = 12;
    pub const MARGIN: i32 = 16;

    /// Separator width (percentage of window width)
    pub const SEPARATOR_WIDTH_RATIO: f64 = 0.8;

    /// Log view minimum height
    pub const LOG_MIN_HEIGHT: i32 = 100;
}

/// System package requirements
pub mod packages {
    /// Required binaries grouped by category
    pub const REQUIRED_BINARIES: &[&str] = &[
        "wipefs",
        "parted",
        "mkfs.vfat",
        "mkfs.ntfs",
        "mount",
        "rsync",
        "dd",
        "sync"
    ];

    /// Optional binaries for enhanced features
    pub const OPTIONAL_BINARIES: &[&str] = &[
        "udisksctl",
        "lsblk"
    ];
}

/// File patterns and extensions
pub mod files {
    /// ISO file extension
    pub const ISO_EXTENSION: &str = "iso";

    /// Windows boot files to detect
    pub const WINDOWS_BOOT_FILES: &[&str] = &["bootmgr", "bootmgr.efi"];

    /// Windows directories to detect
    pub const WINDOWS_DIRS: &[&str] = &["sources", "support"];

    /// Linux boot directories to detect
    pub const LINUX_BOOT_DIRS: &[&str] = &[
        "boot",
        "isolinux",
        "syslinux",
        "grub",
        "grub2"
    ];

    /// Linux live system indicators
    pub const LINUX_LIVE_INDICATORS: &[&str] = &[
        "casper",
        "live",
        "squashfs"
    ];
}