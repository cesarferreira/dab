# dab - Droid Automation Box 📦🤖

> All your automation tools in one box.

A fast, interactive command-line tool for automating and managing your Android device from your computer.

## Features

- 🚀 **Open** installed apps
- 🗑️ **Uninstall** apps you don't need
- 🧹 **Clear** app data and cache
- 💀 **Force kill** stubborn apps
- 📦 **Download APK** files
- 🔍 **Show app info** (version, permissions, etc)
- 🤖 **Show device info** (model, Android version, etc)
- 🌐 **Show network info** (IP, WiFi, etc)
- 📸 **Take screenshots**
- 🎥 **Record screen**
- 🔎 **Searchable app selection** (find your app in a snap)

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

# 🔍 Show app info
dab app-info

# 🤖 Show device info
dab device

# 🌐 Show network info
dab network

# 📸 Take a screenshot
dab screenshot --output /path/to/screen.png

# 🎥 Record the screen
dab record --output /path/to/demo.mp4
```

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (1.70+ recommended)
- [ADB (Android Debug Bridge)](https://developer.android.com/tools/adb) in your PATH
- An Android device or emulator with USB debugging enabled

## Installation 🥓

### From Source
```bash
# Clone the repository
git clone https://github.com/cesarferreira/apm.git
cd apm

# Build and install
cargo install --path .
```

### From crates.io
```bash
cargo install dab
```

## License

MIT

## Ideas
- [ ] Add a command to turn the connection into a wifi connection
- [ ] see adb logcat for the currently focused app
- [ ] see the currently focused app
