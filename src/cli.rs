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
    /// Show app info (version, permissions, etc)
    #[command(name = "app-info")]
    AppInfo,
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
    /// Show crash logs for a specific app
    Crashes {
        /// The package name to find crashes for (optional)
        #[arg(short, long)]
        package: Option<String>,
        /// Show crashes in the last X minutes (default: 10)
        #[arg(short, long, default_value = "10")]
        since: u32,
        /// Use native crash logs instead of ANR logs
        #[arg(short, long)]
        native: bool,
    },
} 