use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use colored::*;
use inquire::Select;
use regex::Regex;
use std::path::PathBuf;
use std::process::{Command, Output};
use which::which;

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
    /// Download APK
    Download {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
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

    fn display_name(&self) -> String {
        format!("{} ({})", self.app_name, self.package_name)
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
        let output = self.run_command(&["devices"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let devices: Vec<String> = stdout
            .lines()
            .skip(1) // Skip the "List of devices attached" header
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "device" {
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
        let output = self.run_command(&["-s", device, "shell", "pm", "list", "packages", "-f", "-3"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let package_regex = Regex::new(r"package:(.+?)=(.+)")?;
        let mut apps = Vec::new();
        
        for line in stdout.lines() {
            if let Some(captures) = package_regex.captures(line) {
                if captures.len() >= 3 {
                    let apk_path = captures.get(1).unwrap().as_str();
                    let package_name = captures.get(2).unwrap().as_str();
                    
                    // Get app name using aapt
                    let app_name = self.get_app_name(device, apk_path).unwrap_or_else(|_| package_name.to_string());
                    
                    apps.push(App::new(package_name, &app_name));
                }
            }
        }
        
        apps.sort_by(|a, b| a.app_name.to_lowercase().cmp(&b.app_name.to_lowercase()));
        Ok(apps)
    }

    fn get_app_name(&self, device: &str, apk_path: &str) -> Result<String> {
        let output = self.run_command(&[
            "-s", device, "shell", "dumpsys", "package", &apk_path.replace("/base.apk", ""),
        ])?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let name_regex = Regex::new(r"targetSdk=\d+\s+(.+?)\s+")?;
        
        if let Some(captures) = name_regex.captures(&stdout) {
            if captures.len() >= 2 {
                return Ok(captures.get(1).unwrap().as_str().to_string());
            }
        }
        
        // If we couldn't extract the name, return the package part of the path
        let path_parts: Vec<&str> = apk_path.split('/').collect();
        if let Some(last_part) = path_parts.last() {
            Ok(last_part.to_string())
        } else {
            Ok(apk_path.to_string())
        }
    }

    fn open_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.run_command(&["-s", device, "shell", "monkey", "-p", package_name, "-c", "android.intent.category.LAUNCHER", "1"])?;
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

    fn download_apk(&self, device: &str, package_name: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
        // First get the path to the APK
        let output = self.run_command(&["-s", device, "shell", "pm", "path", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let path_regex = Regex::new(r"package:(.+)")?;
        let apk_path = path_regex
            .captures(&stdout)
            .ok_or_else(|| anyhow!("Could not find APK path for {}", package_name))?
            .get(1)
            .unwrap()
            .as_str();
        
        // Determine output file path
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
        
        // Pull the APK
        self.run_command(&["-s", device, "pull", apk_path, &output_file.to_string_lossy()])?;
        
        Ok(output_file)
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let adb_client = AdbClient::new()?;
    
    // Get connected devices
    let devices = adb_client.get_device_list()?;
    
    // Select device if more than one
    let device = if devices.len() > 1 {
        let device_select = Select::new("Select device:", devices).prompt()?;
        device_select
    } else {
        devices[0].clone()
    };
    
    // Get installed apps
    println!("{}", "Loading installed apps...".yellow());
    let apps = adb_client.get_installed_apps(&device)?;
    
    // Determine action based on command or interactive menu
    let action = match &cli.command {
        Some(cmd) => cmd,
        None => {
            let options = vec!["Open", "Uninstall", "Clear App Data", "Download APK"];
            let selection = Select::new("Select action:", options).prompt()?;
            
            match selection {
                "Open" => &Commands::Open,
                "Uninstall" => &Commands::Uninstall,
                "Clear App Data" => &Commands::Clear,
                "Download APK" => &Commands::Download { output: None },
                _ => unreachable!(),
            }
        }
    };
    
    // Select an app
    let app_strings: Vec<String> = apps.iter().map(|app| app.display_name()).collect();
    let app_selection = Select::new("Select app:", app_strings.clone()).prompt()?;
    let selected_index = app_strings.iter().position(|s| s == &app_selection).unwrap();
    let selected_app = &apps[selected_index];
    
    // Execute the selected action
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
        Commands::Download { output } => {
            println!("{} APK for {}", "Downloading".cyan(), selected_app.app_name);
            let output_path = adb_client.download_apk(&device, &selected_app.package_name, output.clone())?;
            println!("APK downloaded to {}", output_path.display());
        }
    }
    
    Ok(())
}
