# abd - Android Bacon Dispenser

A fast, interactive command-line tool for managing Android apps and devices from your computer.

## Features

- **Open** installed apps
- **Uninstall** apps
- **Clear** app data and cache
- **Force kill** running apps
- **Download APK** files
- **Show app info** (version, permissions, etc)
- **Show device info** (model, Android version, etc)
- **Take screenshots**
- **Record screen**
- **Searchable app selection**

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (1.70+ recommended)
- [ADB (Android Debug Bridge)](https://developer.android.com/tools/adb) in your PATH
- An Android device or emulator with USB debugging enabled

## Installation

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
cargo install abd
```

## Usage

Run the interactive UI:

```bash
abd
```

Or use direct commands:

```bash
# Open an app
abd open

# Uninstall an app
abd uninstall

# Clear app data
abd clear

# Force kill an app
abd force-kill

# Download APK (optionally specify output path)
abd download
abd download --output /path/to/save.apk

# Show app info
abd app-info

# Show device info
abd device

# Take a screenshot
abd screenshot --output /path/to/screen.png

# Record the screen
abd record --output /path/to/demo.mp4
```

## License

MIT

