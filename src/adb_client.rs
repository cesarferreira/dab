//! Contains the AdbClient struct and its implementation for ADB-related logic.
use std::path::PathBuf;
use std::process::{Command, Output};
use which::which;
use anyhow::{anyhow, Result};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ctrlc;
use colored::*;
use super::app::App;
use std::fs;
use zip::ZipArchive;

pub struct AdbClient {
    pub adb_path: PathBuf,
}

impl AdbClient {
    pub fn new() -> Result<Self> {
        let adb_path = which("adb").map_err(|_| anyhow!("ADB not found in PATH. Please install Android SDK."))?;
        Ok(Self { adb_path })
    }

    pub fn run_command(&self, args: &[&str]) -> Result<Output> {
        let output = Command::new(&self.adb_path)
            .args(args)
            .output()?;
        Ok(output)
    }

    pub fn get_device_list(&self) -> Result<Vec<String>> {
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

    pub fn get_installed_apps(&self, device: &str) -> Result<Vec<App>> {
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

    pub fn get_device_apk_path(&self, device: &str, package_name: &str) -> Result<String> {
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

    pub fn open_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.run_command(&[
            "-s", device, "shell", "monkey", "-p", package_name, 
            "-c", "android.intent.category.LAUNCHER", "1"
        ])?;
        Ok(())
    }

    pub fn uninstall_app(&self, device: &str, package_name: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "uninstall", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Success") {
            Ok(())
        } else {
            Err(anyhow!("Failed to uninstall app: {}", stdout.trim()))
        }
    }

    pub fn clear_app_data(&self, device: &str, package_name: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "pm", "clear", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Success") {
            Ok(())
        } else {
            Err(anyhow!("Failed to clear app data: {}", stdout.trim()))
        }
    }

    pub fn force_kill_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.run_command(&["-s", device, "shell", "am", "force-stop", package_name])?;
        Ok(())
    }

    pub fn download_apk(&self, device: &str, package_name: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
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

    pub fn get_app_info(&self, device: &str, package_name: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "pm", "dump", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
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
        let mut granted_permissions = Vec::new();
        for line in stdout.lines() {
            let trimmed = line.trim();
            if (trimmed.contains("android.permission.") || trimmed.contains("com.android.permission.")) && trimmed.contains("granted=true") {
                let perm = trimmed.split(':').next().unwrap_or("").split_whitespace().next().unwrap_or("");
                if !perm.is_empty() && !granted_permissions.contains(&perm.to_string()) {
                    granted_permissions.push(perm.to_string());
                }
            }
        }
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

    pub fn get_device_info(&self, device: &str) -> Result<()> {
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

    pub fn take_screenshot(&self, device: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
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

    pub fn record_screen(&self, device: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
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
        let pid_file = "/sdcard/screenrecord.pid";
        let start_cmd = format!(
            "screenrecord {} & echo $! > {} && wait $(cat {})",
            remote_path, pid_file, pid_file
        );
        let mut child = Command::new(&self.adb_path)
            .args(["-s", device, "shell", &start_cmd])
            .spawn()?;
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
            let pid_output = Command::new(&adb_path_for_ctrlc)
                .args(["-s", &device_for_ctrlc, "shell", "cat", pid_file])
                .output();
            if let Ok(output) = pid_output {
                if let Ok(pid_str) = String::from_utf8(output.stdout) {
                    let pid = pid_str.trim();
                    if !pid.is_empty() {
                        let _ = Command::new(&adb_path_for_ctrlc)
                            .args(["-s", &device_for_ctrlc, "shell", "kill", "-2", pid])
                            .output();
                    }
                }
            }
        }).expect("Error setting Ctrl-C handler");
        let status = child.wait()?;
        running.store(false, Ordering::SeqCst);
        let _ = self.run_command(&["-s", device, "pull", remote_path, &output_file.to_string_lossy()]);
        let _ = self.run_command(&["-s", device, "shell", "rm", remote_path]);
        let _ = self.run_command(&["-s", device, "shell", "rm", pid_file]);
        println!("Screen recording saved to {}", output_file.display());
        if !status.success() {
            return Err(anyhow!("Screenrecord failed or was interrupted"));
        }
        Ok(output_file)
    }

    pub fn get_network_info(&self, device: &str) -> Result<()> {
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
        let output = self.run_command(&["-s", device, "shell", "dumpsys", "wifi"])?;
        let wifi_info = String::from_utf8_lossy(&output.stdout);
        let mut ssid = None;
        for line in wifi_info.lines() {
            if let Some(idx) = line.find("SSID:") {
                let after = &line[idx + 5..];
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

    pub fn enable_wifi(&self, device: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "ip", "-4", "addr", "show", "wlan0"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut ip_addr = None;
        for line in stdout.lines() {
            if let Some(ip) = line.trim().strip_prefix("inet ") {
                let ip = ip.split_whitespace().next().unwrap_or("");
                let ip_only = ip.split('/').next().unwrap_or("");
                ip_addr = Some(ip_only.to_string());
                break;
            }
        }
        let ip = ip_addr.ok_or_else(|| anyhow!("Could not determine device Wi-Fi IP address. Is Wi-Fi enabled?"))?;
        println!("Enabling ADB over Wi-Fi (TCP/IP 5555)...");
        self.run_command(&["-s", device, "tcpip", "5555"])?;
        println!("Connecting to {}:5555...", ip);
        let connect_output = self.run_command(&["connect", &format!("{}:5555", ip)])?;
        let connect_stdout = String::from_utf8_lossy(&connect_output.stdout);
        println!("{}", connect_stdout.trim());
        println!("\nYou can now disconnect the USB cable and use ADB over Wi-Fi!");
        Ok(())
    }

    pub fn enable_usb(&self, device: &str) -> Result<()> {
        println!("Disconnecting all ADB over network connections...");
        // Run 'adb disconnect' (no -s, global disconnect)
        let _ = self.run_command(&["disconnect"]);
        println!("Switching ADB back to USB mode...");
        self.run_command(&["-s", device, "usb"])?;
        println!("ADB is now in USB mode. If you were connected over Wi-Fi, you may disconnect the Wi-Fi connection.");
        Ok(())
    }

    pub fn get_device_health(&self, device: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "dumpsys", "battery"])?;
        let battery = String::from_utf8_lossy(&output.stdout);
        let mut battery_level = "N/A".to_string();
        let mut battery_status = "N/A".to_string();
        for line in battery.lines() {
            if line.trim().starts_with("level:") {
                battery_level = line.trim().split(':').nth(1).unwrap_or("").trim().to_string();
            }
            if line.trim().starts_with("status:") {
                battery_status = line.trim().split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }
        let output = self.run_command(&["-s", device, "shell", "df", "/data"])?;
        let storage = String::from_utf8_lossy(&output.stdout);
        let mut storage_info = "N/A".to_string();
        for line in storage.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 5 {
                let total_kb = cols[1].replace(",", "").parse::<f64>().unwrap_or(0.0);
                let used_kb = cols[2].replace(",", "").parse::<f64>().unwrap_or(0.0);
                let free_kb = cols[3].replace(",", "").parse::<f64>().unwrap_or(0.0);
                let total_gb = total_kb / 1024.0 / 1024.0;
                let used_gb = used_kb / 1024.0 / 1024.0;
                let free_gb = free_kb / 1024.0 / 1024.0;
                let percent_used = if total_kb > 0.0 { (used_kb / total_kb) * 100.0 } else { 0.0 };
                let percent_free = if total_kb > 0.0 { (free_kb / total_kb) * 100.0 } else { 0.0 };
                storage_info = format!(
                    "Used: {:.2} GB ({:.1}%) / Total: {:.2} GB | Free: {:.2} GB ({:.1}%)",
                    used_gb, percent_used, total_gb, free_gb, percent_free
                );
                break;
            }
        }
        let output = self.run_command(&["-s", device, "shell", "cat", "/proc/meminfo"])?;
        let meminfo = String::from_utf8_lossy(&output.stdout);
        let mut total_ram_kb = None;
        let mut free_ram_kb = None;
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total_ram_kb = line.replace("MemTotal:", "").trim().split_whitespace().next().and_then(|v| v.parse::<f64>().ok());
            }
            if line.starts_with("MemAvailable:") {
                free_ram_kb = line.replace("MemAvailable:", "").trim().split_whitespace().next().and_then(|v| v.parse::<f64>().ok());
            }
        }
        let (total_ram_gb, free_ram_gb) = match (total_ram_kb, free_ram_kb) {
            (Some(total), Some(free)) => (total / 1024.0 / 1024.0, free / 1024.0 / 1024.0),
            _ => (0.0, 0.0),
        };
        let output = self.run_command(&["-s", device, "shell", "ip", "-4", "addr", "show"])?;
        let ip_addr = String::from_utf8_lossy(&output.stdout);
        let mut ip = "N/A".to_string();
        for line in ip_addr.lines() {
            if let Some(ip_line) = line.trim().strip_prefix("inet ") {
                let candidate_ip = ip_line.split_whitespace().next().unwrap_or("").split('/').next().unwrap_or("").to_string();
                if candidate_ip != "127.0.0.1" && !candidate_ip.is_empty() {
                    ip = candidate_ip;
                    break;
                }
            }
        }
        let output = self.run_command(&["-s", device, "shell", "dumpsys", "wifi"])?;
        let wifi_info = String::from_utf8_lossy(&output.stdout);
        let mut ssid = "N/A".to_string();
        for line in wifi_info.lines() {
            if let Some(idx) = line.find("SSID:") {
                let after = &line[idx + 5..];
                let mut ssid_val = after.trim().split(',').next().unwrap_or("").trim().to_string();
                while ssid_val.starts_with('"') || ssid_val.ends_with('"') {
                    ssid_val = ssid_val.trim_matches('"').to_string();
                }
                ssid_val = ssid_val.trim().to_string();
                if !ssid_val.is_empty() && ssid_val != "<unknown ssid>" && ssid_val != "0x0" {
                    ssid = ssid_val;
                    break;
                }
            }
        }
        println!("\n{}", "Device Health Check".bold().underline().yellow());
        println!("{} {}% (Status: {})", "Battery:".cyan(), battery_level.green(), battery_status.green());
        println!("{} {}", "Storage:".cyan(), storage_info.green());
        println!("{} {:.2} GB free / {:.2} GB total", "RAM:".cyan(), free_ram_gb, total_ram_gb);
        println!("{} {} (SSID: {})", "Network:".cyan(), ip.green(), ssid.green());
        Ok(())
    }

    pub fn launch_url(&self, device: &str, url: &str) -> Result<()> {
        let output = self.run_command(&["-s", device, "shell", "am", "start", "-a", "android.intent.action.VIEW", "-d", url])?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("{} {}", "Error launching URL:".red(), stderr.red());
        }
        Ok(())
    }

    pub fn grant_permissions(&self, device: &str, package_name: &str, permissions: &[&str]) -> Result<()> {
        for &permission in permissions {
            let output = self.run_command(&["-s", device, "shell", "pm", "grant", package_name, permission])?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                eprintln!("Error granting {}: {}", permission, stderr.red());
            }
        }
        Ok(())
    }

    pub fn revoke_permissions(&self, device: &str, package_name: &str, permissions: &[&str]) -> Result<()> {
        for &permission in permissions {
            let output = self.run_command(&["-s", device, "shell", "pm", "revoke", package_name, permission])?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                eprintln!("Error revoking {}: {}", permission, stderr.red());
            }
        }
        Ok(())
    }

    pub fn install_file(&self, device: &str, file_path: &PathBuf) -> Result<()> {
        // Check if file exists
        if !file_path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path.display()));
        }

        // Check file extension to determine type
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("apk") => {
                println!("{} {}", "Installing APK:".green(), file_path.display());
                self.install_apk(device, file_path)
            }
            Some("xapk") => {
                println!("{} {}", "Installing XAPK:".green(), file_path.display());
                self.install_xapk(device, file_path)
            }
            _ => {
                Err(anyhow!("Unsupported file type. Only APK and XAPK files are supported."))
            }
        }
    }

    fn install_apk(&self, device: &str, apk_path: &PathBuf) -> Result<()> {
        let output = self.run_command(&["-s", device, "install", "-d", &apk_path.to_string_lossy()])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stdout.contains("Success") {
            println!("{}", "APK installed successfully!".green());
            Ok(())
        } else {
            eprintln!("{} {}", "Error installing APK:".red(), stderr.red());
            Err(anyhow!("Failed to install APK: {}", stderr.trim()))
        }
    }

    fn install_xapk(&self, device: &str, xapk_path: &PathBuf) -> Result<()> {
        // Create temporary directory
        let temp_dir = std::env::temp_dir().join(format!("dab_xapk_{}", 
            std::process::id()));
        fs::create_dir_all(&temp_dir)?;

        // Extract XAPK file
        println!("{} {}", "Extracting XAPK to:".yellow(), temp_dir.display());
        let file = fs::File::open(xapk_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        // Find all APK files in the extracted directory
        let mut apk_files = Vec::new();
        self.find_apk_files(&temp_dir, &mut apk_files)?;

        if apk_files.is_empty() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(anyhow!("No APK files found in XAPK"));
        }

        // Install multiple APKs
        println!("{} {} APK files", "Installing".green(), apk_files.len());
        let mut args = vec!["-s", device, "install-multiple", "-d"];
        let apk_paths: Vec<String> = apk_files.iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();
        let apk_path_refs: Vec<&str> = apk_paths.iter().map(|s| s.as_str()).collect();
        args.extend(apk_path_refs);

        let output = self.run_command(&args)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Clean up temporary directory
        let _ = fs::remove_dir_all(&temp_dir);

        if stdout.contains("Success") {
            println!("{}", "XAPK installed successfully!".green());
            Ok(())
        } else {
            eprintln!("{} {}", "Error installing XAPK:".red(), stderr.red());
            Err(anyhow!("Failed to install XAPK: {}", stderr.trim()))
        }
    }

    fn find_apk_files(&self, dir: &PathBuf, apk_files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.find_apk_files(&path, apk_files)?;
            } else if path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase()) == Some("apk".to_string()) {
                apk_files.push(path);
            }
        }
        Ok(())
    }

    pub fn analyze_local_file(&self, file_path: &PathBuf) -> Result<()> {
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("apk") => self.analyze_apk(file_path),
            Some("xapk") => self.analyze_xapk(file_path),
            _ => Err(anyhow!("Unsupported file type. Only APK and XAPK files are supported.")),
        }
    }

    fn analyze_apk(&self, apk_path: &PathBuf) -> Result<()> {
        // Try to use aapt first
        match self.analyze_apk_with_aapt(apk_path) {
            Ok(_) => return Ok(()),
            Err(_) => {
                println!("{}", "aapt not found, using basic ZIP analysis...".yellow());
            }
        }

        // Fallback to basic ZIP analysis
        self.analyze_apk_basic(apk_path)
    }

    fn analyze_apk_with_aapt(&self, apk_path: &PathBuf) -> Result<()> {
        // Try aapt first, then aapt2
        let aapt_commands = ["aapt", "aapt2"];
        
        for &aapt_cmd in &aapt_commands {
            if let Ok(aapt_path) = which::which(aapt_cmd) {
                let output = Command::new(&aapt_path)
                    .args(["dump", "badging", &apk_path.to_string_lossy()])
                    .output()?;
                
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    self.parse_aapt_output(&stdout)?;
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("aapt not available"))
    }

    fn parse_aapt_output(&self, aapt_output: &str) -> Result<()> {
        // Debug mode: show raw aapt output
        if std::env::var("DAB_DEBUG").is_ok() {
            println!("{}", "\n=== RAW AAPT OUTPUT ===".yellow());
            println!("{}", aapt_output);
            println!("{}", "=== END RAW OUTPUT ===\n".yellow());
        }

        let mut package_name = "N/A".to_string();
        let mut version_code = "N/A".to_string();
        let mut version_name = "N/A".to_string();
        let mut app_name = "N/A".to_string();
        let mut permissions = Vec::new();

        for line in aapt_output.lines() {
            let line = line.trim();
            
            if line.starts_with("package:") {
                // Parse: package: name='com.example.app' versionCode='1' versionName='1.0'
                if let Some(name_start) = line.find("name='") {
                    if let Some(name_end) = line[name_start + 6..].find("'") {
                        package_name = line[name_start + 6..name_start + 6 + name_end].to_string();
                    }
                }
                if let Some(code_start) = line.find("versionCode='") {
                    if let Some(code_end) = line[code_start + 13..].find("'") {
                        version_code = line[code_start + 13..code_start + 13 + code_end].to_string();
                    }
                }
                // Try multiple patterns for versionName
                if let Some(name_start) = line.find("versionName='") {
                    if let Some(name_end) = line[name_start + 13..].find("'") {
                        let extracted_version = line[name_start + 13..name_start + 13 + name_end].to_string();
                        version_name = if extracted_version.is_empty() { 
                            "Not set".to_string() 
                        } else { 
                            extracted_version 
                        };
                    }
                } else if let Some(name_start) = line.find("versionName=\"") {
                    // Handle double quotes instead of single quotes
                    if let Some(name_end) = line[name_start + 13..].find("\"") {
                        let extracted_version = line[name_start + 13..name_start + 13 + name_end].to_string();
                        version_name = if extracted_version.is_empty() { 
                            "Not set".to_string() 
                        } else { 
                            extracted_version 
                        };
                    }
                }
            } else if line.starts_with("application-label:") {
                app_name = line.replace("application-label:", "").trim().trim_matches('\'').trim_matches('"').to_string();
                if app_name.is_empty() {
                    app_name = "N/A".to_string();
                }
            } else if line.starts_with("application-label-") {
                // Handle localized labels like application-label-en:'App Name'
                if app_name == "N/A" {
                    let label = line.split(':').nth(1).unwrap_or("").trim().trim_matches('\'').trim_matches('"').to_string();
                    if !label.is_empty() {
                        app_name = label;
                    }
                }
            } else if line.starts_with("uses-permission:") {
                if let Some(perm_start) = line.find("name='") {
                    if let Some(perm_end) = line[perm_start + 6..].find("'") {
                        let permission = line[perm_start + 6..perm_start + 6 + perm_end].to_string();
                        if !permissions.contains(&permission) {
                            permissions.push(permission);
                        }
                    }
                } else if let Some(perm_start) = line.find("name=\"") {
                    if let Some(perm_end) = line[perm_start + 6..].find("\"") {
                        let permission = line[perm_start + 6..perm_start + 6 + perm_end].to_string();
                        if !permissions.contains(&permission) {
                            permissions.push(permission);
                        }
                    }
                }
            }
        }

        println!("{}", "\nAPK File Analysis".bold().underline().yellow());
        println!("{}: {}", "Package Name".cyan(), package_name.green());
        println!("{}: {}", "App Name".cyan(), app_name.green());
        println!("{}: {}", "Version Code".cyan(), version_code.green());
        println!("{}: {}", "Version Name".cyan(), version_name.green());
        println!("{}:", "Permissions Requested".cyan());
        if permissions.is_empty() {
            println!("  {}", "None".red());
        } else {
            for perm in permissions {
                println!("  {}", perm.blue());
            }
        }

        Ok(())
    }

    fn analyze_apk_basic(&self, apk_path: &PathBuf) -> Result<()> {
        let file = fs::File::open(apk_path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let mut has_manifest = false;
        let mut classes_dex_count = 0;
        let mut assets_count = 0;
        let mut res_count = 0;
        
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let name = file.name();
            
            if name == "AndroidManifest.xml" {
                has_manifest = true;
            } else if name.starts_with("classes") && name.ends_with(".dex") {
                classes_dex_count += 1;
            } else if name.starts_with("assets/") {
                assets_count += 1;
            } else if name.starts_with("res/") {
                res_count += 1;
            }
        }

        println!("{}", "\nAPK File Analysis (Basic)".bold().underline().yellow());
        println!("{}: {}", "File Path".cyan(), apk_path.display().to_string().green());
        println!("{}: {}", "Has AndroidManifest.xml".cyan(), if has_manifest { "Yes".green() } else { "No".red() });
        println!("{}: {}", "DEX Files".cyan(), classes_dex_count.to_string().green());
        println!("{}: {}", "Asset Files".cyan(), assets_count.to_string().green());
        println!("{}: {}", "Resource Files".cyan(), res_count.to_string().green());
        println!("{}: {}", "Total Files".cyan(), archive.len().to_string().green());
        
        // Try to get file size
        if let Ok(metadata) = fs::metadata(apk_path) {
            let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
            println!("{}: {:.2} MB", "File Size".cyan(), size_mb.to_string().green());
        }

        println!("\n{}", "Note: For detailed app info (package name, version, permissions), install 'aapt' or 'aapt2' from Android SDK.".yellow());

        Ok(())
    }

    fn analyze_xapk(&self, xapk_path: &PathBuf) -> Result<()> {
        // Create temporary directory
        let temp_dir = std::env::temp_dir().join(format!("dab_xapk_analysis_{}", 
            std::process::id()));
        fs::create_dir_all(&temp_dir)?;

        // Extract XAPK file
        println!("{} {}", "Extracting XAPK to:".yellow(), temp_dir.display());
        let file = fs::File::open(xapk_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        // Find all APK files in the extracted directory
        let mut apk_files = Vec::new();
        self.find_apk_files(&temp_dir, &mut apk_files)?;

        if apk_files.is_empty() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(anyhow!("No APK files found in XAPK"));
        }

        println!("{} {} APK files found in XAPK", "Found".green(), apk_files.len());
        
        // Try to find the base APK (main app APK)
        let base_apk = self.find_base_apk(&apk_files)?;
        
        println!("{} {}", "Analyzing base APK:".yellow(), base_apk.file_name().unwrap_or_default().to_string_lossy());
        
        // Analyze the base APK
        self.analyze_apk(&base_apk)?;

        // Show info about other APKs if debug mode is enabled
        if std::env::var("DAB_DEBUG").is_ok() {
            println!("\n{}", "Other APK files in XAPK:".cyan());
            for apk_file in &apk_files {
                if apk_file != &base_apk {
                    let file_size = fs::metadata(apk_file)
                        .map(|m| format!("{:.1} MB", m.len() as f64 / 1024.0 / 1024.0))
                        .unwrap_or_else(|_| "Unknown".to_string());
                    println!("  {} ({})", apk_file.file_name().unwrap_or_default().to_string_lossy().blue(), file_size);
                }
            }
        }

        // Clean up temporary directory
        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    fn find_base_apk(&self, apk_files: &[PathBuf]) -> Result<PathBuf> {
        // Strategy 1: Look for base.apk
        for apk_file in apk_files {
            if let Some(filename) = apk_file.file_name() {
                if filename.to_string_lossy().to_lowercase() == "base.apk" {
                    return Ok(apk_file.clone());
                }
            }
        }
        
        // Strategy 2: Look for APK files with "base" in the name
        for apk_file in apk_files {
            if let Some(filename) = apk_file.file_name() {
                let name = filename.to_string_lossy().to_lowercase();
                if name.contains("base") {
                    return Ok(apk_file.clone());
                }
            }
        }
        
        // Strategy 3: Find the largest APK (likely the main app)
        let mut largest_apk = apk_files[0].clone();
        let mut largest_size = 0u64;
        
        for apk_file in apk_files {
            if let Ok(metadata) = fs::metadata(apk_file) {
                let size = metadata.len();
                if size > largest_size {
                    largest_size = size;
                    largest_apk = apk_file.clone();
                }
            }
        }
        
        Ok(largest_apk)
    }
} 