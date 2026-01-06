//! Contains CLI argument parsing structs and enums.
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Open an app
    Open,
    /// Uninstall an app
    Uninstall,
    /// Clear app data
    Clear,
    /// Force kill an app
    #[command(name = "force-kill")]
    ForceKill,
    /// Download APK
    Download {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show app info (version, etc)
    #[command(name = "app-info")]
    AppInfo {
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
    Grant,
    /// Revoke permissions from an app
    Revoke,
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
