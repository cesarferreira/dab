<div align="center">

# dab - Droid Automation Box 📦🤖

<p>
  <a href="https://crates.io/crates/dab-cli"><img src="https://img.shields.io/crates/v/dab-cli.svg" alt="crates.io version" /></a>
  <a href="https://crates.io/crates/dab-cli"><img src="https://img.shields.io/crates/d/dab-cli.svg" alt="crates.io downloads" /></a>
  <a href="https://crates.io/crates/dab-cli"><img src="https://img.shields.io/crates/l/dab-cli.svg" alt="license" /></a>
</p>

<p>
A fast, interactive command-line tool for automating and managing your Android device from your computer.
</p>
<img src="extras/t-rec.gif" alt="demo" width="100%" />
</div>



## Features

- 🚀 **Open** installed apps
- 🗑️ **Uninstall** apps you don't need
- 🧹 **Clear** app data and cache
- 💀 **Force kill** stubborn apps
- 📦 **Download APK** files
- 📲 **Install APK/XAPK/APKM** files from your computer
- 🔍 **Show app info** (version, use `--all` for permissions)
- 📄 **Analyze local APK/XAPK/APKM** files without installation
- 🛡️ **Grant or revoke app permissions** (multi-select from known permissions)
- 🤖 **Show device info** (model, Android version, etc)
- 🌐 **Show network info** (IP, WiFi, etc)
- 🩺 **Device Health Check** (battery, storage, RAM, network)
- 📶 **Enable ADB over Wi-Fi** (connect wirelessly to your device)
- 🔌 **Switch ADB back to USB mode** (revert to cable connection)
- 📸 **Take screenshots**
- 🎥 **Record screen**
- 🔎 **Searchable app selection** (find your app in a snap)
- 🚀 **Launch** URLs or deep links in your Android device (open YouTube, browser, or any app via deep link)

## Usage

Run the interactive UI:

```bash
dab
```

Or use direct commands:

```bash
# 🚀 Open an app
dab open

# 🗑️ Uninstall an app
dab uninstall

# 🧹 Clear app data
dab clear

# 💀 Force kill an app
dab force-kill

# 📦 Download APK (optionally specify output path)
dab download
dab download --output /path/to/save.apk

# 📲 Install APK, XAPK, or APKM file
dab install /path/to/app.apk
dab install /path/to/app.xapk
dab install /path/to/app.apkm

# 📄 Analyze local APK, XAPK, or APKM file (no device needed)
dab info /path/to/app.apk
dab info /path/to/app.xapk
dab info /path/to/app.apkm

# 🔍 Show app info
dab app-info
dab app-info --all   # include permissions (-a)

# 🛡️ Grant permissions to an app (multi-select)
dab grant

# 🛡️ Revoke permissions from an app (multi-select)
dab revoke

# 🤖 Show device info
dab device

# 🌐 Show network info
dab network

# 📸 Take a screenshot
dab screenshot --output /path/to/screen.png

# 🎥 Record the screen
dab record --output /path/to/demo.mp4

# 📶 Enable ADB over Wi-Fi (no more cables!)
dab wifi

# 🔌 Switch ADB back to USB mode
dab usb

# 🩺 Device Health Check (battery, storage, RAM, network)
dab health

# 🚀 Launch a URL or deep link
dab launch <URL>
```

Example:

```sh
$ dab launch https://cesarferreira.com                     # URL that opens in your default browser
$ dab launch recipes://recipe/12345                        # DEEP LINK to the "recipes app"
$ dab launch https://www.youtube.com/watch?v=dQw4w9WgXcQ   # opens youtube
$ dab launch wathever you want                             # urls that deep link, apps, wathever
```

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (1.70+ recommended)
- [ADB (Android Debug Bridge)](https://developer.android.com/tools/adb) in your PATH
- An Android device or emulator with USB debugging enabled

## Installation 🥓

### From crates.io
```bash
cargo install dab-cli
```

### From Source
```bash
# Clone the repository
git clone https://github.com/cesarferreira/dab.git
cd dab

# Build and install
cargo install --path .
```

## License

MIT
