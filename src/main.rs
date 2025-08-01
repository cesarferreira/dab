mod cli;
mod app;
mod adb_client;

use anyhow::Result;
use clap::Parser;
use colored::*;
use inquire::{Select, MultiSelect};
use cli::{Cli, Commands};
use adb_client::AdbClient;

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    let adb_client = AdbClient::new()?;
    
    // Handle commands that don't require device connection first
    match &cli.command {
        Some(Commands::Info { file }) => {
            println!("{} {}", "Analyzing file:".yellow(), file.display());
            adb_client.analyze_local_file(file)?;
            return Ok(());
        },
        _ => {}
    }
    
    // Define common Android permissions once
    let android_permissions = vec![
        "android.permission.CAMERA",
        "android.permission.RECORD_AUDIO",
        "android.permission.READ_CONTACTS",
        "android.permission.WRITE_CONTACTS",
        "android.permission.GET_ACCOUNTS",
        "android.permission.ACCESS_FINE_LOCATION",
        "android.permission.ACCESS_COARSE_LOCATION",
        "android.permission.ACCESS_BACKGROUND_LOCATION",
        "android.permission.READ_PHONE_STATE",
        "android.permission.CALL_PHONE",
        "android.permission.READ_CALL_LOG",
        "android.permission.WRITE_CALL_LOG",
        "android.permission.ADD_VOICEMAIL",
        "android.permission.USE_SIP",
        "android.permission.BODY_SENSORS",
        "android.permission.SEND_SMS",
        "android.permission.RECEIVE_SMS",
        "android.permission.READ_SMS",
        "android.permission.RECEIVE_WAP_PUSH",
        "android.permission.RECEIVE_MMS",
        "android.permission.READ_EXTERNAL_STORAGE",
        "android.permission.WRITE_EXTERNAL_STORAGE",
        "android.permission.INTERNET",
    ];
    
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
        Some(Commands::Wifi) => {
            println!("{}", "Setting up ADB over Wi-Fi...".yellow());
            adb_client.enable_wifi(&device)?;
            return Ok(());
        },
        Some(Commands::Usb) => {
            println!("{}", "Switching ADB to USB mode...".yellow());
            adb_client.enable_usb(&device)?;
            return Ok(());
        },
        Some(Commands::Health) => {
            println!("{}", "Checking device health...".yellow());
            adb_client.get_device_health(&device)?;
            return Ok(());
        },
        Some(Commands::Launch { url }) => {
            println!("{} {}", "Launching:".green(), url.cyan());
            adb_client.launch_url(&device, url)?;
            return Ok(());
        },
        Some(Commands::Install { file }) => {
            println!("{} {}", "Installing file:".yellow(), file.display());
            adb_client.install_file(&device, file)?;
            return Ok(());
        },
        Some(Commands::Grant) => {
            println!("{}", "Granting permissions...".yellow());
            let apps = adb_client.get_installed_apps(&device)?;
            let app_strings: Vec<String> = apps.iter().map(|app| app.package_name.clone()).collect();
            let app_selection = Select::new("Select app:", app_strings.clone()).with_page_size(15).prompt()?;
            let selected_index = app_strings.iter().position(|s| s == &app_selection).unwrap();
            let selected_app = &apps[selected_index];
            
            let selected = MultiSelect::new("Select permissions to grant (space to select, enter to apply):", android_permissions.clone())
                .with_page_size(15)
                .prompt()?;
            if selected.is_empty() {
                println!("No permissions selected.");
            } else {
                let perms: Vec<&str> = selected.iter().map(|s| &**s).collect();
                adb_client.grant_permissions(&device, &selected_app.package_name, &perms)?;
                println!("Permissions granted successfully.");
            }
            return Ok(());
        },
        Some(Commands::Revoke) => {
            println!("{}", "Revoking permissions...".yellow());
            let apps = adb_client.get_installed_apps(&device)?;
            let app_strings: Vec<String> = apps.iter().map(|app| app.package_name.clone()).collect();
            let app_selection = Select::new("Select app:", app_strings.clone()).with_page_size(15).prompt()?;
            let selected_index = app_strings.iter().position(|s| s == &app_selection).unwrap();
            let selected_app = &apps[selected_index];
            
            let selected = MultiSelect::new("Select permissions to revoke (space to select, enter to apply):", android_permissions.clone())
                .with_page_size(15)
                .prompt()?;
            if selected.is_empty() {
                println!("No permissions selected.");
            } else {
                let perms: Vec<&str> = selected.iter().map(|s| &**s).collect();
                adb_client.revoke_permissions(&device, &selected_app.package_name, &perms)?;
                println!("Permissions revoked successfully.");
            }
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
    let app_strings: Vec<String> = apps.iter().map(|app| app.package_name.clone()).collect();
    let app_selection = Select::new("Select app:", app_strings.clone()).with_page_size(15).prompt()?;
    let selected_index = app_strings.iter().position(|s| s == &app_selection).unwrap();
    let selected_app = &apps[selected_index];
    let action = match &cli.command {
        Some(cmd) => cmd,
        None => {
            let options = vec!["Open", "App Info", "Uninstall", "Clear App Data", "Force Kill", "Download APK", "Grant Permissions", "Revoke Permissions"];
            let selection = Select::new("Select action:", options).prompt()?;
            match selection {
                "Open" => &Commands::Open,
                "App Info" => &Commands::AppInfo,
                "Uninstall" => &Commands::Uninstall,
                "Clear App Data" => &Commands::Clear,
                "Force Kill" => &Commands::ForceKill,
                "Download APK" => &Commands::Download { output: None },
                "Grant Permissions" => &Commands::Grant,
                "Revoke Permissions" => &Commands::Revoke,
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
        Commands::Wifi => {
            println!("{}", "Setting up ADB over Wi-Fi...".yellow());
            adb_client.enable_wifi(&device)?;
            return Ok(());
        }
        Commands::Usb => {
            println!("{}", "Switching ADB to USB mode...".yellow());
            adb_client.enable_usb(&device)?;
            return Ok(());
        }
        Commands::Health => {
            println!("{}", "Checking device health...".yellow());
            adb_client.get_device_health(&device)?;
            return Ok(());
        }
        Commands::Launch { .. } => {
            unreachable!("Launch command should be handled earlier and never reach this point");
        }
        Commands::Grant => {
            println!("{}", "Granting permissions...".yellow());
            
            let selected = MultiSelect::new("Select permissions to grant (space to select, enter to apply):", android_permissions.clone())
                .with_page_size(15)
                .prompt()?;
            if selected.is_empty() {
                println!("No permissions selected.");
            } else {
                let perms: Vec<&str> = selected.iter().map(|s| &**s).collect();
                adb_client.grant_permissions(&device, &selected_app.package_name, &perms)?;
                println!("Permissions granted successfully.");
            }
        }
        Commands::Revoke => {
            println!("{}", "Revoking permissions...".yellow());
            
            let selected = MultiSelect::new("Select permissions to revoke (space to select, enter to apply):", android_permissions.clone())
                .with_page_size(15)
                .prompt()?;
            if selected.is_empty() {
                println!("No permissions selected.");
            } else {
                let perms: Vec<&str> = selected.iter().map(|s| &**s).collect();
                adb_client.revoke_permissions(&device, &selected_app.package_name, &perms)?;
                println!("Permissions revoked successfully.");
            }
        }
        Commands::Install { .. } => {
            unreachable!("Install command should be handled earlier and never reach this point");
        }
        Commands::Info { .. } => {
            unreachable!("Info command should be handled earlier and never reach this point");
        }
    }
    Ok(())
}

fn main() {
    match real_main() {
        Ok(()) => {},
        Err(e) => {
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
