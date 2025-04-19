use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use std::path::PathBuf;
use std::process::{Command, Output};
use which::which;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ctrlc;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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
}

struct App {
    package_name: String,
    app_name: String,
}

impl App {
    fn new(package_name: &str, app_name: &str) -> Self {
        Self {
            package_name: package_name.to_string(),
            app_name: app_name.to_string(),
        }
    }
}

struct AdbClient {
    adb_path: PathBuf,
}

impl AdbClient {
    fn new() -> Result<Self> {
        let adb_path = which("adb").map_err(|_| anyhow!("ADB not found in PATH. Please install Android SDK."))?;
        Ok(Self { adb_path })
    }

    fn run_command(&self, args: &[&str]) -> Result<Output> {
        let output = Command::new(&self.adb_path)
            .args(args)
            .output()?;
        
        Ok(output)
    }

    fn get_device_list(&self) -> Result<Vec<String>> {
        let output = self.run_command(&["devices", "-l"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let devices: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter(|line| !line.contains("daemon not running"))
            .filter(|line| !line.contains("daemon started"))
            .filter(|line| !line.contains("List of devices attached"))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    Some(parts[0].to_string())
                } else {
                    None
                }
            })
            .collect();
        
        if devices.is_empty() {
            return Err(anyhow!("No connected devices found. Please connect an Android device via USB and enable USB debugging."));
        }
        
        Ok(devices)
    }

    fn get_installed_apps(&self, device: &str) -> Result<Vec<App>> {
        let output = self.run_command(&["-s", device, "shell", "pm", "list", "packages"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut package_names: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.replace("package:", "").trim().to_string())
            .collect();
        package_names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        let apps = package_names
            .into_iter()
            .map(|package_name| App::new(&package_name, &package_name))
            .collect();
        Ok(apps)
    }

    fn get_device_apk_path(&self, device: &str, package_name: &str) -> Result<String> {
        let output = self.run_command(&["-s", device, "shell", "pm", "list", "packages", "-f"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let apk_path = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.replace("package:", ""))
            .find(|line| line.trim().ends_with(package_name.trim()))
            .map(|line| line.replace(&format!("={}", package_name), ""));
        
        apk_path.ok_or_else(|| anyhow!("Could not find APK path for {}", package_name))
    }

    fn open_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.run_command(&[
            "-s", device, "shell", "monkey", "-p", package_name, 
            "-c", "android.intent.category.LAUNCHER", "1"
        ])?;
        Ok(())
    }

    fn uninstall_app(&self, device: &str, package_name: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "uninstall", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        if stdout.contains("Success") {
            Ok(())
        } else {
            Err(anyhow!("Failed to uninstall app: {}", stdout.trim()))
        }
    }

    fn clear_app_data(&self, device: &str, package_name: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "pm", "clear", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        if stdout.contains("Success") {
            Ok(())
        } else {
            Err(anyhow!("Failed to clear app data: {}", stdout.trim()))
        }
    }

    fn force_kill_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.run_command(&["-s", device, "shell", "am", "force-stop", package_name])?;
        Ok(())
    }

    fn download_apk(&self, device: &str, package_name: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
        let apk_path = self.get_device_apk_path(device, package_name)?;
        
        let output_file = match output_path {
            Some(path) => {
                if path.is_dir() {
                    path.join(format!("{}.apk", package_name))
                } else {
                    path
                }
            },
            None => {
                std::env::current_dir()?.join(format!("{}.apk", package_name))
            }
        };
        
        println!("Downloading APK to {}", output_file.display());
        
        self.run_command(&["-s", device, "pull", &apk_path, &output_file.to_string_lossy()])?;
        
        Ok(output_file)
    }

    fn get_app_info(&self, device: &str, package_name: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "pm", "dump", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse version code and version name
        let version_code = stdout.lines().find_map(|line| {
            if line.trim().starts_with("versionCode=") {
                line.trim().split('=').nth(1).map(|s| s.split_whitespace().next().unwrap_or("").to_string())
            } else {
                None
            }
        }).unwrap_or_else(|| "N/A".to_string());

        let version_name = stdout.lines().find_map(|line| {
            if line.trim().starts_with("versionName=") {
                line.trim().split('=').nth(1).map(|s| s.to_string())
            } else {
                None
            }
        }).unwrap_or_else(|| "N/A".to_string());

        // Find all permissions with granted=true
        let mut granted_permissions = Vec::new();
        for line in stdout.lines() {
            let trimmed = line.trim();
            if (trimmed.contains("android.permission.") || trimmed.contains("com.android.permission.")) && trimmed.contains("granted=true") {
                // Extract the permission name (before the colon or space)
                let perm = trimmed.split(':').next().unwrap_or("").split_whitespace().next().unwrap_or("");
                if !perm.is_empty() && !granted_permissions.contains(&perm.to_string()) {
                    granted_permissions.push(perm.to_string());
                }
            }
        }

        // Print info in a colored table
        println!("{}", "\nApp Info".bold().underline().yellow());
        println!("{}: {}", "Package Name".cyan(), package_name.green());
        println!("{}: {}", "Version Code".cyan(), version_code.green());
        println!("{}: {}", "Version Name".cyan(), version_name.green());
        println!("{}:", "Granted Permissions".cyan());
        if granted_permissions.is_empty() {
            println!("  {}", "None".red());
        } else {
            for perm in granted_permissions {
                println!("  {}", perm.blue());
            }
        }
        Ok(())
    }

    fn get_device_info(&self, device: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "getprop"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut info = std::collections::HashMap::new();
        let relevant_keys = [
            "ro.product.model",
            "ro.product.manufacturer",
            "ro.product.brand",
            "ro.product.device",
            "ro.product.name",
            "ro.build.version.release",
            "ro.build.version.sdk",
            "ro.build.version.codename",
            "ro.product.board",
            "ro.product.cpu.abi",
            "ro.product.locale",
            "ro.build.id",
            "ro.build.version.security_patch",
        ];
        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once("]: [") {
                let key = key.trim_start_matches('[');
                let value = value.trim_end_matches(']');
                if relevant_keys.contains(&key) {
                    info.insert(key, value);
                }
            }
        }
        println!("\n{}", "Device Info".bold().underline().yellow());
        for &key in &relevant_keys {
            let label = match key {
                "ro.product.model" => "Model",
                "ro.product.manufacturer" => "Manufacturer",
                "ro.product.brand" => "Brand",
                "ro.product.device" => "Device",
                "ro.product.name" => "Name",
                "ro.build.version.release" => "Android Version",
                "ro.build.version.sdk" => "SDK",
                "ro.build.version.codename" => "Codename",
                "ro.product.board" => "Board",
                "ro.product.cpu.abi" => "CPU ABI",
                "ro.product.locale" => "Locale",
                "ro.build.id" => "Build ID",
                "ro.build.version.security_patch" => "Security Patch",
                _ => key,
            };
            if let Some(val) = info.get(key) {
                println!("{:<18}: {}", label.cyan(), val.green());
            }
        }
        Ok(())
    }

    fn take_screenshot(&self, device: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
        let remote_path = "/sdcard/screen.png";
        let output_file = match output_path {
            Some(path) => {
                if path.is_dir() {
                    path.join("screen.png")
                } else {
                    path
                }
            },
            None => {
                std::env::current_dir()?.join("screen.png")
            }
        };
        self.run_command(&["-s", device, "shell", "screencap", "-p", remote_path])?;
        self.run_command(&["-s", device, "pull", remote_path, &output_file.to_string_lossy()])?;
        println!("Screenshot saved to {}", output_file.display());
        Ok(output_file)
    }

    fn record_screen(&self, device: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
        let remote_path = "/sdcard/demo.mp4";
        let output_file = match output_path {
            Some(path) => {
                if path.is_dir() {
                    path.join("demo.mp4")
                } else {
                    path
                }
            },
            None => {
                std::env::current_dir()?.join("demo.mp4")
            }
        };
        println!("Recording... Press Ctrl+C to stop.");
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let device_for_ctrlc = device.to_string();
        let adb_path_for_ctrlc = self.adb_path.clone();
        // We'll store the child process id on the device
        let pid_file = "/sdcard/screenrecord.pid";
        // Start screenrecord in the background and save its PID
        let start_cmd = format!(
            "screenrecord {} & echo $! > {} && wait $(cat {})",
            remote_path, pid_file, pid_file
        );
        let mut child = Command::new(&self.adb_path)
            .args(["-s", device, "shell", &start_cmd])
            .spawn()?;
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
            // Read the PID and send SIGINT to it
            let pid_output = Command::new(&adb_path_for_ctrlc)
                .args(["-s", &device_for_ctrlc, "shell", "cat", pid_file])
                .output();
            if let Ok(output) = pid_output {
                if let Ok(pid_str) = String::from_utf8(output.stdout) {
                    let pid = pid_str.trim();
                    if !pid.is_empty() {
                        let _ = Command::new(&adb_path_for_ctrlc)
                            .args(["-s", &device_for_ctrlc, "shell", "kill", "-2", pid]) // -2 is SIGINT
                            .output();
                    }
                }
            }
        }).expect("Error setting Ctrl-C handler");
        let status = child.wait()?;
        running.store(false, Ordering::SeqCst);
        // Always try to pull the file, even if interrupted
        let _ = self.run_command(&["-s", device, "pull", remote_path, &output_file.to_string_lossy()]);
        // Remove the file from the device
        let _ = self.run_command(&["-s", device, "shell", "rm", remote_path]);
        let _ = self.run_command(&["-s", device, "shell", "rm", pid_file]);
        println!("Screen recording saved to {}", output_file.display());
        if !status.success() {
            return Err(anyhow!("Screenrecord failed or was interrupted"));
        }
        Ok(output_file)
    }

    fn get_network_info(&self, device: &str) -> Result<()> {
        // Get IP addresses (ipv4 only, no subnet)
        let output = self.run_command(&["-s", device, "shell", "ip", "-4", "addr", "show"])?;
        let ip_addr = String::from_utf8_lossy(&output.stdout);
        println!("\n{}", "Network Interfaces (IP addresses)".bold().underline().yellow());
        for line in ip_addr.lines() {
            if let Some(ip) = line.trim().strip_prefix("inet ") {
                let ip = ip.split_whitespace().next().unwrap_or("");
                let ip_only = ip.split('/').next().unwrap_or("");
                println!("{} {}", "IP Address:".cyan(), ip_only.green());
            }
        }

        // Get WiFi info (only SSID network name, stripped of quotes and extra text)
        let output = self.run_command(&["-s", device, "shell", "dumpsys", "wifi"])?;
        let wifi_info = String::from_utf8_lossy(&output.stdout);
        let mut ssid = None;
        for line in wifi_info.lines() {
            if let Some(idx) = line.find("SSID:") {
                let after = &line[idx + 5..];
                // Remove all leading/trailing quotes and whitespace
                let mut ssid_val = after.trim().split(',').next().unwrap_or("").trim().to_string();
                while ssid_val.starts_with('"') || ssid_val.ends_with('"') {
                    ssid_val = ssid_val.trim_matches('"').to_string();
                }
                ssid_val = ssid_val.trim().to_string();
                if !ssid_val.is_empty() && ssid_val != "<unknown ssid>" && ssid_val != "0x0" {
                    ssid = Some(ssid_val);
                    break;
                }
            }
        }
        println!("\n{}", "WiFi Info".bold().underline().yellow());
        println!("{} {}", "SSID:".cyan(), ssid.unwrap_or_else(|| "N/A".to_string()).green());
        Ok(())
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    let adb_client = AdbClient::new()?;
    let devices = adb_client.get_device_list()?;
    let device = if devices.len() > 1 {
        let device_select = Select::new("Select device:", devices).prompt()?;
        device_select
    } else {
        devices[0].clone()
    };
    match &cli.command {
        Some(Commands::Device) => {
            println!("{}", "Fetching device info...".yellow());
            adb_client.get_device_info(&device)?;
            return Ok(());
        },
        Some(Commands::Network) => {
            println!("{}", "Fetching network info...".yellow());
            adb_client.get_network_info(&device)?;
            return Ok(());
        },
        Some(Commands::Screenshot { output }) => {
            println!("{}", "Taking screenshot...".yellow());
            adb_client.take_screenshot(&device, output.clone())?;
            return Ok(());
        },
        Some(Commands::Record { output }) => {
            println!("{}", "Recording screen...".yellow());
            adb_client.record_screen(&device, output.clone())?;
            return Ok(());
        },
        _ => {}
    }
    println!("{}", "Loading installed apps...".yellow());
    let apps = adb_client.get_installed_apps(&device)?;
    if apps.is_empty() {
        println!("{}", "No installed apps found.".yellow());
        return Ok(());
    }
    // Show searchable app picker
    let app_strings: Vec<String> = apps.iter().map(|app| app.package_name.clone()).collect();
    let app_selection = Select::new("Select app:", app_strings.clone()).with_page_size(15).prompt()?;
    let selected_index = app_strings.iter().position(|s| s == &app_selection).unwrap();
    let selected_app = &apps[selected_index];
    // Now show the action picker
    let action = match &cli.command {
        Some(cmd) => cmd,
        None => {
            let options = vec!["Open", "App Info", "Uninstall", "Clear App Data", "Force Kill", "Download APK"];
            let selection = Select::new("Select action:", options).prompt()?;
            match selection {
                "Open" => &Commands::Open,
                "App Info" => &Commands::AppInfo,
                "Uninstall" => &Commands::Uninstall,
                "Clear App Data" => &Commands::Clear,
                "Force Kill" => &Commands::ForceKill,
                "Download APK" => &Commands::Download { output: None },
                _ => unreachable!(),
            }
        }
    };
    match action {
        Commands::Open => {
            println!("{} {}", "Opening".green(), selected_app.app_name);
            adb_client.open_app(&device, &selected_app.package_name)?;
        }
        Commands::Uninstall => {
            println!("{} {}", "Uninstalling".red(), selected_app.app_name);
            adb_client.uninstall_app(&device, &selected_app.package_name)?;
        }
        Commands::Clear => {
            println!("{} data for {}", "Clearing".blue(), selected_app.app_name);
            adb_client.clear_app_data(&device, &selected_app.package_name)?;
        }
        Commands::ForceKill => {
            println!("{} {}", "Force killing".red(), selected_app.app_name);
            adb_client.force_kill_app(&device, &selected_app.package_name)?;
        }
        Commands::Download { output } => {
            println!("{} APK for {}", "Downloading".cyan(), selected_app.app_name);
            let output_path = adb_client.download_apk(&device, &selected_app.package_name, output.clone())?;
            println!("APK downloaded to {}", output_path.display());
        }
        Commands::AppInfo => {
            println!("{} {}", "Fetching info for".yellow(), selected_app.app_name);
            adb_client.get_app_info(&device, &selected_app.package_name)?;
        }
        Commands::Device => {
            println!("{}", "Fetching device info...".yellow());
            adb_client.get_device_info(&device)?;
        }
        Commands::Screenshot { output } => {
            println!("{}", "Taking screenshot...".yellow());
            adb_client.take_screenshot(&device, output.clone())?;
        }
        Commands::Record { output } => {
            println!("{}", "Recording screen...".yellow());
            adb_client.record_screen(&device, output.clone())?;
        }
        Commands::Network => {
            println!("{}", "Fetching network info...".yellow());
            adb_client.get_network_info(&device)?;
        }
    }
    Ok(())
}

fn main() {
    match real_main() {
        Ok(()) => {},
        Err(e) => {
            // Check for inquire interruption
            if let Some(inquire_err) = e.downcast_ref::<inquire::InquireError>() {
                if matches!(inquire_err, inquire::InquireError::OperationInterrupted) {
                    std::process::exit(0);
                }
            }
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
