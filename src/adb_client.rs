//! `AdbClient` — dab's presentation layer over the shared `androkit` toolkit.
//!
//! The ADB/APK plumbing that used to live here now lives in `androkit`
//! (`androkit::adb`, `androkit::apk`), so dab and `adev` share one
//! implementation. This file keeps dab's public method surface, colored output,
//! and exact JSON shapes intact — it just delegates the real work to androkit
//! and reshapes the results.

use super::app::App;
use androkit::adb::Adb;
use androkit::apk;
use anyhow::{anyhow, Result};
use colored::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Constants for android permissions
pub mod permission {
    pub const CAMERA: &str = "android.permission.CAMERA";
    pub const RECORD_AUDIO: &str = "android.permission.RECORD_AUDIO";
    pub const READ_CONTACTS: &str = "android.permission.READ_CONTACTS";
    pub const WRITE_CONTACTS: &str = "android.permission.WRITE_CONTACTS";
    pub const GET_ACCOUNTS: &str = "android.permission.GET_ACCOUNTS";
    pub const ACCESS_FINE_LOCATION: &str = "android.permission.ACCESS_FINE_LOCATION";
    pub const ACCESS_COARSE_LOCATION: &str = "android.permission.ACCESS_COARSE_LOCATION";
    pub const ACCESS_BACKGROUND_LOCATION: &str = "android.permission.ACCESS_BACKGROUND_LOCATION";
    pub const READ_PHONE_STATE: &str = "android.permission.READ_PHONE_STATE";
    pub const CALL_PHONE: &str = "android.permission.CALL_PHONE";
    pub const READ_CALL_LOG: &str = "android.permission.READ_CALL_LOG";
    pub const WRITE_CALL_LOG: &str = "android.permission.WRITE_CALL_LOG";
    pub const ADD_VOICEMAIL: &str = "android.permission.ADD_VOICEMAIL";
    pub const USE_SIP: &str = "android.permission.USE_SIP";
    pub const BODY_SENSORS: &str = "android.permission.BODY_SENSORS";
    pub const SEND_SMS: &str = "android.permission.SEND_SMS";
    pub const RECEIVE_SMS: &str = "android.permission.RECEIVE_SMS";
    pub const READ_SMS: &str = "android.permission.READ_SMS";
    pub const RECEIVE_WAP_PUSH: &str = "android.permission.RECEIVE_WAP_PUSH";
    pub const RECEIVE_MMS: &str = "android.permission.RECEIVE_MMS";
    pub const READ_EXTERNAL_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
    pub const WRITE_EXTERNAL_STORAGE: &str = "android.permission.WRITE_EXTERNAL_STORAGE";
    pub const INTERNET: &str = "android.permission.INTERNET";
}

pub struct AdbClient {
    adb: Adb,
}

impl AdbClient {
    pub fn new() -> Result<Self> {
        Ok(Self { adb: Adb::new()? })
    }

    // ── devices ──────────────────────────────────────────────────────────

    pub fn get_device_list(&self) -> Result<Vec<String>> {
        self.adb.devices()
    }

    pub fn get_device_list_json(&self) -> Value {
        match self.adb.devices() {
            Ok(devices) => json!({ "devices": devices }),
            Err(e) => json!({ "error": e.to_string(), "devices": [] }),
        }
    }

    // ── apps ─────────────────────────────────────────────────────────────

    pub fn get_installed_apps(&self, device: &str) -> Result<Vec<App>> {
        Ok(self
            .adb
            .list_packages(device)?
            .into_iter()
            .map(|p| App::new(&p))
            .collect())
    }

    pub fn get_installed_apps_json(&self, device: &str) -> Value {
        match self.adb.list_packages(device) {
            Ok(apps) => json!({ "device": device, "apps": apps }),
            Err(e) => json!({ "error": e.to_string(), "apps": [] }),
        }
    }

    pub fn open_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.adb.launch_package(device, package_name)
    }

    pub fn uninstall_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.adb.uninstall(device, package_name)
    }

    pub fn clear_app_data(&self, device: &str, package_name: &str) -> Result<()> {
        self.adb.clear_data(device, package_name)
    }

    pub fn force_kill_app(&self, device: &str, package_name: &str) -> Result<()> {
        self.adb.stop_app(device, package_name)
    }

    pub fn download_apk(
        &self,
        device: &str,
        package_name: &str,
        output_path: Option<PathBuf>,
    ) -> Result<PathBuf> {
        self.adb.download_apk(device, package_name, output_path)
    }

    // ── app info (pm dump — dab-specific introspection) ──────────────────

    pub fn get_app_info(
        &self,
        device: &str,
        package_name: &str,
        show_permissions: bool,
    ) -> Result<()> {
        let output = self
            .adb
            .run(&["-s", device, "shell", "pm", "dump", package_name])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (version_code, version_name) = parse_versions(&stdout);
        println!("{}", "\nApp Info".bold().underline().yellow());
        println!("{}: {}", "Package Name".cyan(), package_name.green());
        println!("{}: {}", "Version Code".cyan(), version_code.green());
        println!("{}: {}", "Version Name".cyan(), version_name.green());
        if show_permissions {
            let granted = parse_granted_permissions(&stdout);
            println!("{}:", "Granted Permissions".cyan());
            if granted.is_empty() {
                println!("  {}", "None".red());
            } else {
                for perm in granted {
                    println!("  {}", perm.blue());
                }
            }
        }
        Ok(())
    }

    pub fn get_app_info_json(
        &self,
        device: &str,
        package_name: &str,
        include_permissions: bool,
    ) -> Value {
        let output = match self
            .adb
            .run(&["-s", device, "shell", "pm", "dump", package_name])
        {
            Ok(o) => o,
            Err(e) => return json!({ "error": e.to_string() }),
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (version_code, version_name) = parse_versions(&stdout);
        let mut result = json!({
            "package_name": package_name,
            "version_code": version_code,
            "version_name": version_name,
        });
        if include_permissions {
            result["granted_permissions"] = json!(parse_granted_permissions(&stdout));
        }
        result
    }

    // ── device info ──────────────────────────────────────────────────────

    pub fn get_device_info(&self, device: &str) -> Result<()> {
        let info = self.adb.device_info(device)?;
        println!("\n{}", "Device Info".bold().underline().yellow());
        let rows: [(&str, &Option<String>); 13] = [
            ("Model", &info.model),
            ("Manufacturer", &info.manufacturer),
            ("Brand", &info.brand),
            ("Device", &info.device),
            ("Name", &info.name),
            ("Android Version", &info.android_version),
            ("SDK", &info.sdk),
            ("Codename", &info.codename),
            ("Board", &info.board),
            ("CPU ABI", &info.cpu_abi),
            ("Locale", &info.locale),
            ("Build ID", &info.build_id),
            ("Security Patch", &info.security_patch),
        ];
        for (label, value) in rows {
            if let Some(v) = value {
                println!("{:<18}: {}", label.cyan(), v.green());
            }
        }
        Ok(())
    }

    pub fn get_device_info_json(&self, device: &str) -> Value {
        match self.adb.device_info(device) {
            Ok(info) => {
                serde_json::to_value(info).unwrap_or_else(|e| json!({ "error": e.to_string() }))
            }
            Err(e) => json!({ "error": e.to_string() }),
        }
    }

    // ── network ──────────────────────────────────────────────────────────

    pub fn get_network_info(&self, device: &str) -> Result<()> {
        let info = self.adb.network_info(device)?;
        println!(
            "\n{}",
            "Network Interfaces (IP addresses)"
                .bold()
                .underline()
                .yellow()
        );
        for ip in &info.ip_addresses {
            println!("{} {}", "IP Address:".cyan(), ip.green());
        }
        println!("\n{}", "WiFi Info".bold().underline().yellow());
        println!(
            "{} {}",
            "SSID:".cyan(),
            info.ssid.unwrap_or_else(|| "N/A".to_string()).green()
        );
        Ok(())
    }

    pub fn get_network_info_json(&self, device: &str) -> Value {
        match self.adb.network_info(device) {
            Ok(info) => json!({
                "device": info.device,
                "ip_addresses": info.ip_addresses,
                "ssid": info.ssid.map(Value::from).unwrap_or(Value::Null),
            }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }

    // ── health ───────────────────────────────────────────────────────────

    pub fn get_device_health(&self, device: &str) -> Result<()> {
        let h = self.adb.device_health(device)?;
        println!("\n{}", "Device Health Check".bold().underline().yellow());
        println!(
            "{} {}% (Status: {})",
            "Battery:".cyan(),
            h.battery.level.unwrap_or_else(|| "N/A".to_string()).green(),
            h.battery
                .status
                .unwrap_or_else(|| "N/A".to_string())
                .green()
        );
        if let Some(s) = h.storage {
            println!(
                "{} Used: {:.2} GB ({:.1}%) / Total: {:.2} GB | Free: {:.2} GB",
                "Storage:".cyan(),
                s.used_gb,
                s.percent_used,
                s.total_gb,
                s.free_gb
            );
        }
        println!(
            "{} {:.2} GB free / {:.2} GB total",
            "RAM:".cyan(),
            h.ram.free_gb,
            h.ram.total_gb
        );
        println!(
            "{} {} (SSID: {})",
            "Network:".cyan(),
            h.network.ip.unwrap_or_else(|| "N/A".to_string()).green(),
            h.network.ssid.unwrap_or_else(|| "N/A".to_string()).green()
        );
        Ok(())
    }

    pub fn get_device_health_json(&self, device: &str) -> Value {
        match self.adb.device_health(device) {
            Ok(h) => json!({
                "device": h.device,
                "battery": {
                    "level": h.battery.level.map(Value::from).unwrap_or(Value::Null),
                    "status": h.battery.status.map(Value::from).unwrap_or(Value::Null),
                },
                "storage": h.storage.map(|s| json!({
                    "total_gb": s.total_gb,
                    "used_gb": s.used_gb,
                    "free_gb": s.free_gb,
                    "percent_used": s.percent_used,
                })).unwrap_or(Value::Null),
                "ram": { "total_gb": h.ram.total_gb, "free_gb": h.ram.free_gb },
                "network": {
                    "ip": h.network.ip.map(Value::from).unwrap_or(Value::Null),
                    "ssid": h.network.ssid.map(Value::from).unwrap_or(Value::Null),
                },
            }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }

    // ── media ────────────────────────────────────────────────────────────

    pub fn take_screenshot(&self, device: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
        let path = self.adb.screenshot(device, output_path)?;
        println!("Screenshot saved to {}", path.display());
        Ok(path)
    }

    pub fn record_screen(&self, device: &str, output_path: Option<PathBuf>) -> Result<PathBuf> {
        println!("Recording... Press Ctrl+C to stop.");
        let path = self.adb.record_screen(device, output_path)?;
        println!("Screen recording saved to {}", path.display());
        Ok(path)
    }

    // ── connectivity ─────────────────────────────────────────────────────

    pub fn enable_wifi(&self, device: &str) -> Result<()> {
        println!("Enabling ADB over Wi-Fi (TCP/IP 5555)...");
        let addr = self.adb.enable_wifi(device)?;
        println!("Connected to {}", addr.green());
        println!("\nYou can now disconnect the USB cable and use ADB over Wi-Fi!");
        Ok(())
    }

    pub fn enable_usb(&self, device: &str) -> Result<()> {
        println!("Switching ADB back to USB mode...");
        self.adb.enable_usb(device)?;
        println!("ADB is now in USB mode.");
        Ok(())
    }

    // ── launch & permissions ─────────────────────────────────────────────

    pub fn launch_url(&self, device: &str, url: &str) -> Result<()> {
        self.adb.launch_url(device, url)
    }

    pub fn grant_permissions(
        &self,
        device: &str,
        package_name: &str,
        permissions: &[&str],
    ) -> Result<()> {
        self.adb.grant(device, package_name, permissions)
    }

    pub fn revoke_permissions(
        &self,
        device: &str,
        package_name: &str,
        permissions: &[&str],
    ) -> Result<()> {
        self.adb.revoke(device, package_name, permissions)
    }

    // ── install ──────────────────────────────────────────────────────────

    pub fn install_file(&self, device: &str, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            return Err(anyhow!("File does not exist: {}", file_path.display()));
        }
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        match extension.as_deref() {
            Some("apk") => {
                println!("{} {}", "Installing APK:".green(), file_path.display());
                self.adb.install_apk(device, file_path)?;
                println!("{}", "APK installed successfully!".green());
                Ok(())
            }
            Some("xapk") | Some("apkm") => {
                println!(
                    "{} {}",
                    "Installing XAPK/APKM:".green(),
                    file_path.display()
                );
                let temp_dir =
                    std::env::temp_dir().join(format!("dab_xapk_{}", std::process::id()));
                std::fs::create_dir_all(&temp_dir)?;
                let result = (|| {
                    let apks = apk::extract_apks(file_path, &temp_dir)?;
                    if apks.is_empty() {
                        return Err(anyhow!("No APK files found in XAPK"));
                    }
                    println!("{} {} APK files", "Installing".green(), apks.len());
                    self.adb.install_multiple(device, &apks)?;
                    println!("{}", "XAPK installed successfully!".green());
                    Ok(())
                })();
                let _ = std::fs::remove_dir_all(&temp_dir);
                result
            }
            _ => Err(anyhow!(
                "Unsupported file type. Only APK, XAPK, and APKM files are supported."
            )),
        }
    }

    // ── local file analysis ──────────────────────────────────────────────

    pub fn analyze_local_file(&self, file_path: &Path) -> Result<()> {
        let info = apk::analyze(file_path)?;
        println!("{}", "\nAPK File Analysis".bold().underline().yellow());
        println!("{}: {}", "Package Name".cyan(), info.package_name.green());
        println!("{}: {}", "App Name".cyan(), info.app_name.green());
        println!("{}: {}", "Version Code".cyan(), info.version_code.green());
        println!("{}: {}", "Version Name".cyan(), info.version_name.green());
        println!("{}:", "Permissions Requested".cyan());
        if info.permissions.is_empty() {
            println!("  {}", "None".red());
        } else {
            for perm in &info.permissions {
                println!("  {}", perm.blue());
            }
        }
        Ok(())
    }

    pub fn analyze_local_file_json(&self, file_path: &Path) -> Value {
        match apk::analyze(file_path) {
            Ok(info) => {
                serde_json::to_value(info).unwrap_or_else(|e| json!({ "error": e.to_string() }))
            }
            Err(e) => json!({ "error": e.to_string() }),
        }
    }
}

/// Extract `versionCode` / `versionName` from `pm dump` output.
fn parse_versions(dump: &str) -> (String, String) {
    let version_code = dump
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("versionCode=")
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
        })
        .unwrap_or_else(|| "N/A".to_string());
    let version_name = dump
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("versionName=")
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "N/A".to_string());
    (version_code, version_name)
}

/// Extract granted permission names from `pm dump` output.
fn parse_granted_permissions(dump: &str) -> Vec<String> {
    let mut granted = Vec::new();
    for line in dump.lines() {
        let trimmed = line.trim();
        if (trimmed.contains("android.permission.") || trimmed.contains("com.android.permission."))
            && trimmed.contains("granted=true")
        {
            let perm = trimmed
                .split(':')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !perm.is_empty() && !granted.contains(&perm.to_string()) {
                granted.push(perm.to_string());
            }
        }
    }
    granted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_versions_extracts_code_and_name() {
        let dump = "\
    Packages:
      Package [com.example.app] (a1b2c3):
        versionCode=12345 minSdk=21 targetSdk=33
        versionName=1.2.3
        flags=[ ... ]";
        let (code, name) = parse_versions(dump);
        assert_eq!(code, "12345");
        assert_eq!(name, "1.2.3");
    }

    #[test]
    fn parse_versions_falls_back_to_na_when_missing() {
        let dump = "\
    Packages:
      Package [com.example.app] (a1b2c3):
        flags=[ ... ]
        dataDir=/data/user/0/com.example.app";
        let (code, name) = parse_versions(dump);
        assert_eq!(code, "N/A");
        assert_eq!(name, "N/A");
    }

    #[test]
    fn parse_granted_permissions_returns_granted_deduped_in_order() {
        let dump = format!(
            "\
    requested permissions:
            {}
            {}
    install permissions:
            {}: granted=true
            {}: granted=false
      com.android.permission.SPECIAL: granted=true
            {}: granted=true
            {}: granted=true",
            permission::INTERNET,
            permission::CAMERA,
            permission::INTERNET,
            permission::CAMERA,
            permission::INTERNET,
            permission::ACCESS_FINE_LOCATION
        );
        let granted = parse_granted_permissions(&dump);
        assert_eq!(
            granted,
            vec![
                permission::INTERNET.to_string(),
                "com.android.permission.SPECIAL".to_string(),
                permission::ACCESS_FINE_LOCATION.to_string(),
            ]
        );
    }

    #[test]
    fn parse_granted_permissions_empty_when_none_granted() {
        let dump = format!(
            "\
            {}: granted=false
            {}: granted=false",
            permission::INTERNET,
            permission::CAMERA
        );
        assert!(parse_granted_permissions(&dump).is_empty());
    }
}
