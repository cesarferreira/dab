mod adb_client;
mod app;
mod cli;

use adb_client::AdbClient;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use colored::*;
use inquire::{MultiSelect, Select};

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    let adb_client = AdbClient::new()?;
    let json = cli.json;

    // ── Commands that don't need a connected device ──────────────────────────

    // `dab devices` — list connected devices
    if matches!(&cli.command, Some(Commands::Devices)) {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&adb_client.get_device_list_json())?
            );
        } else {
            let devices = adb_client.get_device_list()?;
            println!("{}", "Connected devices:".bold().yellow());
            for d in devices {
                println!("  {}", d.green());
            }
        }
        return Ok(());
    }

    // `dab info <file>` — analyze local APK/XAPK/APKM
    if let Some(Commands::Info { file }) = &cli.command {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&adb_client.analyze_local_file_json(file))?
            );
        } else {
            println!("{} {}", "Analyzing file:".yellow(), file.display());
            adb_client.analyze_local_file(file)?;
        }
        return Ok(());
    }

    // ── Select (or pin) device ───────────────────────────────────────────────

    let device: String = if let Some(serial) = &cli.device {
        serial.clone()
    } else {
        let devices = adb_client.get_device_list()?;
        if devices.len() > 1 {
            Select::new("Select device:", devices).prompt()?
        } else {
            devices.into_iter().next().unwrap()
        }
    };

    // ── Commands that need a device but not an app ───────────────────────────

    match &cli.command {
        Some(Commands::Apps) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&adb_client.get_installed_apps_json(&device))?
                );
            } else {
                let apps = adb_client.get_installed_apps(&device)?;
                println!("{}", "Installed apps:".bold().yellow());
                for app in apps {
                    println!("  {}", app.package_name);
                }
            }
            return Ok(());
        }
        Some(Commands::Device) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&adb_client.get_device_info_json(&device))?
                );
            } else {
                println!("{}", "Fetching device info...".yellow());
                adb_client.get_device_info(&device)?;
            }
            return Ok(());
        }
        Some(Commands::Network) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&adb_client.get_network_info_json(&device))?
                );
            } else {
                println!("{}", "Fetching network info...".yellow());
                adb_client.get_network_info(&device)?;
            }
            return Ok(());
        }
        Some(Commands::Health) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&adb_client.get_device_health_json(&device))?
                );
            } else {
                println!("{}", "Checking device health...".yellow());
                adb_client.get_device_health(&device)?;
            }
            return Ok(());
        }
        Some(Commands::Screenshot { output }) => {
            let path = adb_client.take_screenshot(&device, output.clone())?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "output": path.to_string_lossy() })
                );
            }
            return Ok(());
        }
        Some(Commands::Record { output }) => {
            println!("{}", "Recording screen...".yellow());
            let path = adb_client.record_screen(&device, output.clone())?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "output": path.to_string_lossy() })
                );
            }
            return Ok(());
        }
        Some(Commands::Wifi) => {
            println!("{}", "Setting up ADB over Wi-Fi...".yellow());
            adb_client.enable_wifi(&device)?;
            if json {
                println!("{}", serde_json::json!({ "success": true }));
            }
            return Ok(());
        }
        Some(Commands::Usb) => {
            println!("{}", "Switching ADB to USB mode...".yellow());
            adb_client.enable_usb(&device)?;
            if json {
                println!("{}", serde_json::json!({ "success": true }));
            }
            return Ok(());
        }
        Some(Commands::Launch { url }) => {
            if !json {
                println!("{} {}", "Launching:".green(), url.cyan());
            }
            adb_client.launch_url(&device, url)?;
            if json {
                println!("{}", serde_json::json!({ "success": true, "url": url }));
            }
            return Ok(());
        }
        Some(Commands::Install { file }) => {
            if !json {
                println!("{} {}", "Installing file:".yellow(), file.display());
            }
            adb_client.install_file(&device, file)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "file": file.to_string_lossy() })
                );
            }
            return Ok(());
        }
        _ => {}
    }

    // ── Commands that need a device AND an app ───────────────────────────────

    let android_permissions = vec![
        // "android.permission.CAMERA",
        adb_client::permission::CAMERA,
        adb_client::permission::RECORD_AUDIO,
        adb_client::permission::READ_CONTACTS,
        adb_client::permission::WRITE_CONTACTS,
        adb_client::permission::GET_ACCOUNTS,
        adb_client::permission::ACCESS_FINE_LOCATION,
        adb_client::permission::ACCESS_COARSE_LOCATION,
        adb_client::permission::ACCESS_BACKGROUND_LOCATION,
        adb_client::permission::READ_PHONE_STATE,
        adb_client::permission::CALL_PHONE,
        adb_client::permission::READ_CALL_LOG,
        adb_client::permission::WRITE_CALL_LOG,
        adb_client::permission::ADD_VOICEMAIL,
        adb_client::permission::USE_SIP,
        adb_client::permission::BODY_SENSORS,
        adb_client::permission::SEND_SMS,
        adb_client::permission::RECEIVE_SMS,
        adb_client::permission::READ_SMS,
        adb_client::permission::RECEIVE_WAP_PUSH,
        adb_client::permission::RECEIVE_MMS,
        adb_client::permission::READ_EXTERNAL_STORAGE,
        adb_client::permission::WRITE_EXTERNAL_STORAGE,
        adb_client::permission::INTERNET,
    ];

    // Extract --package from the command, if provided
    let package_flag: Option<&str> = match &cli.command {
        Some(Commands::Open { package }) => package.as_deref(),
        Some(Commands::Uninstall { package }) => package.as_deref(),
        Some(Commands::Clear { package }) => package.as_deref(),
        Some(Commands::ForceKill { package }) => package.as_deref(),
        Some(Commands::Download { package, .. }) => package.as_deref(),
        Some(Commands::AppInfo { package, .. }) => package.as_deref(),
        Some(Commands::Grant { package, .. }) => package.as_deref(),
        Some(Commands::Revoke { package, .. }) => package.as_deref(),
        _ => None,
    };

    // Resolve the target package: flag → interactive picker → full interactive UI
    let selected_package: String = if let Some(pkg) = package_flag {
        pkg.to_string()
    } else if cli.command.is_some() {
        // A subcommand was given but no --package: show app picker
        if !json {
            println!("{}", "Loading installed apps...".yellow());
        }
        let apps = adb_client.get_installed_apps(&device)?;
        if apps.is_empty() {
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "error": "No installed apps found" })
                );
            } else {
                println!("{}", "No installed apps found.".yellow());
            }
            return Ok(());
        }
        let app_strings: Vec<String> = apps.iter().map(|a| a.package_name.clone()).collect();
        Select::new("Select app:", app_strings)
            .with_page_size(15)
            .prompt()?
    } else {
        // No subcommand at all: full interactive UI
        println!("{}", "Loading installed apps...".yellow());
        let apps = adb_client.get_installed_apps(&device)?;
        if apps.is_empty() {
            println!("{}", "No installed apps found.".yellow());
            return Ok(());
        }
        let app_strings: Vec<String> = apps.iter().map(|a| a.package_name.clone()).collect();
        Select::new("Select app:", app_strings)
            .with_page_size(15)
            .prompt()?
    };

    // When no subcommand was given, show the action menu
    let effective_command: Commands = match cli.command {
        Some(command) => command,
        None => {
            let options = vec![
                "Open",
                "App Info",
                "Uninstall",
                "Clear App Data",
                "Force Kill",
                "Download APK",
                "Grant Permissions",
                "Revoke Permissions",
            ];
            let selection = Select::new("Select action:", options).prompt()?;
            match selection {
                "Open" => Commands::Open { package: None },
                "App Info" => Commands::AppInfo {
                    package: None,
                    all: false,
                },
                "Uninstall" => Commands::Uninstall { package: None },
                "Clear App Data" => Commands::Clear { package: None },
                "Force Kill" => Commands::ForceKill { package: None },
                "Download APK" => Commands::Download {
                    package: None,
                    output: None,
                },
                "Grant Permissions" => Commands::Grant {
                    package: None,
                    permissions: None,
                },
                "Revoke Permissions" => Commands::Revoke {
                    package: None,
                    permissions: None,
                },
                _ => unreachable!(),
            }
        }
    };

    match &effective_command {
        Commands::Open { .. } => {
            if !json {
                println!("{} {}", "Opening".green(), selected_package);
            }
            adb_client.open_app(&device, &selected_package)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "package": selected_package })
                );
            }
        }
        Commands::Uninstall { .. } => {
            if !json {
                println!("{} {}", "Uninstalling".red(), selected_package);
            }
            adb_client.uninstall_app(&device, &selected_package)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "package": selected_package })
                );
            }
        }
        Commands::Clear { .. } => {
            if !json {
                println!("{} data for {}", "Clearing".blue(), selected_package);
            }
            adb_client.clear_app_data(&device, &selected_package)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "package": selected_package })
                );
            }
        }
        Commands::ForceKill { .. } => {
            if !json {
                println!("{} {}", "Force killing".red(), selected_package);
            }
            adb_client.force_kill_app(&device, &selected_package)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "package": selected_package })
                );
            }
        }
        Commands::Download { output, .. } => {
            if !json {
                println!("{} APK for {}", "Downloading".cyan(), selected_package);
            }
            let output_path =
                adb_client.download_apk(&device, &selected_package, output.clone())?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "success": true, "package": selected_package, "output": output_path.to_string_lossy() })
                );
            } else {
                println!("APK downloaded to {}", output_path.display());
            }
        }
        Commands::AppInfo { all, .. } => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&adb_client.get_app_info_json(
                        &device,
                        &selected_package,
                        *all
                    ))?
                );
            } else {
                println!("{} {}", "Fetching info for".yellow(), selected_package);
                adb_client.get_app_info(&device, &selected_package, *all)?;
            }
        }
        Commands::Grant { permissions, .. } => {
            let perms_to_grant: Vec<String> = if let Some(p) = permissions {
                p.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                let selected = MultiSelect::new(
                    "Select permissions to grant (space to select, enter to apply):",
                    android_permissions.clone(),
                )
                .with_page_size(15)
                .prompt()?;
                selected.iter().map(|s| s.to_string()).collect()
            };
            if perms_to_grant.is_empty() {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": false, "reason": "no permissions selected" })
                    );
                } else {
                    println!("No permissions selected.");
                }
            } else {
                let perm_refs: Vec<&str> = perms_to_grant.iter().map(|s| s.as_str()).collect();
                adb_client.grant_permissions(&device, &selected_package, &perm_refs)?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": true, "package": selected_package, "granted": perms_to_grant })
                    );
                } else {
                    println!("Permissions granted successfully.");
                }
            }
        }
        Commands::Revoke { permissions, .. } => {
            let perms_to_revoke: Vec<String> = if let Some(p) = permissions {
                p.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                let selected = MultiSelect::new(
                    "Select permissions to revoke (space to select, enter to apply):",
                    android_permissions.clone(),
                )
                .with_page_size(15)
                .prompt()?;
                selected.iter().map(|s| s.to_string()).collect()
            };
            if perms_to_revoke.is_empty() {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": false, "reason": "no permissions selected" })
                    );
                } else {
                    println!("No permissions selected.");
                }
            } else {
                let perm_refs: Vec<&str> = perms_to_revoke.iter().map(|s| s.as_str()).collect();
                adb_client.revoke_permissions(&device, &selected_package, &perm_refs)?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "success": true, "package": selected_package, "revoked": perms_to_revoke })
                    );
                } else {
                    println!("Permissions revoked successfully.");
                }
            }
        }
        // All remaining commands are handled above and should never reach here
        _ => {}
    }

    Ok(())
}

fn main() {
    match real_main() {
        Ok(()) => {}
        Err(e) => {
            if let Some(inquire_err) = e.downcast_ref::<inquire::InquireError>() {
                if matches!(inquire_err, inquire::InquireError::OperationInterrupted) {
                    std::process::exit(0);
                }
            }
            // Emit JSON errors when --json was requested
            let args: Vec<String> = std::env::args().collect();
            if args.iter().any(|a| a == "--json") {
                eprintln!("{}", serde_json::json!({ "error": e.to_string() }));
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    }
}
