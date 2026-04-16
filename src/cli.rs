//! Contains CLI argument parsing structs and enums.
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output results as JSON — useful for AI agents and scripting
    #[arg(long, global = true)]
    pub json: bool,

    /// Target a specific device by serial number, skipping interactive selection
    #[arg(long, global = true, value_name = "SERIAL")]
    pub device: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List connected ADB devices
    Devices,
    /// List all installed apps on the device
    Apps,
    /// Open an app
    Open {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
    },
    /// Uninstall an app
    Uninstall {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
    },
    /// Clear app data
    Clear {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
    },
    /// Force kill an app
    #[command(name = "force-kill")]
    ForceKill {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
    },
    /// Download APK
    Download {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show app info (version, etc)
    #[command(name = "app-info")]
    AppInfo {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
        /// Include permissions and other details
        #[arg(short, long)]
        all: bool,
    },
    /// Show device info (model, manufacturer, Android version, etc)
    Device,
    /// Take a screenshot of the device
    Screenshot {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Record the device screen
    Record {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show network info (IP, WiFi, etc)
    Network,
    /// Enable ADB over Wi-Fi
    Wifi,
    /// Switch ADB back to USB mode
    Usb,
    /// Device health check (battery, storage, RAM, network)
    Health,
    /// Launch a URL or deep link in the Android device
    Launch {
        /// The URL or deep link to launch
        url: String,
    },
    /// Grant permissions to an app
    Grant {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
        /// Comma-separated list of permissions — skips interactive selection
        #[arg(long, value_name = "PERMISSIONS")]
        permissions: Option<String>,
    },
    /// Revoke permissions from an app
    Revoke {
        /// Package name — skips interactive selection
        #[arg(long, value_name = "PACKAGE")]
        package: Option<String>,
        /// Comma-separated list of permissions — skips interactive selection
        #[arg(long, value_name = "PERMISSIONS")]
        permissions: Option<String>,
    },
    /// Install an APK, XAPK, or APKM file
    Install {
        /// Path to the APK, XAPK, or APKM file to install
        file: PathBuf,
    },
    /// Show info for a local APK, XAPK, or APKM file
    Info {
        /// Path to the APK, XAPK, or APKM file to analyze
        file: PathBuf,
    },
}
