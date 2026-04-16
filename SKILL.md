# dab — AI Agent Skill

`dab` (Droid Automation Box) is a CLI tool for automating and managing Android devices over ADB.
This skill teaches AI agents how to use `dab` non-interactively.

## Installation

```bash
cargo install dab-cli
```

Requires `adb` in your `PATH` and a connected Android device (USB or Wi-Fi) with USB debugging enabled.

## Agent-Friendly Flags

| Flag | Description |
|------|-------------|
| `--json` | Emit structured JSON output instead of colored human text |
| `--device <SERIAL>` | Target a specific device by serial, skipping interactive selection |
| `--package <PACKAGE>` | Target a specific app by package name, skipping the app picker |
| `--permissions <LIST>` | Comma-separated permissions for `grant`/`revoke`, skipping the multi-select |

## Quick Reference

### 1. Discover devices

```bash
dab devices --json
```

```json
{
  "devices": ["emulator-5554", "R38M3049YJH"]
}
```

### 2. List installed apps

```bash
dab apps --device emulator-5554 --json
```

```json
{
  "device": "emulator-5554",
  "apps": ["com.android.chrome", "com.example.myapp", "..."]
}
```

### 3. Get device info

```bash
dab device --device emulator-5554 --json
```

```json
{
  "serial": "emulator-5554",
  "model": "sdk_gphone64_arm64",
  "manufacturer": "Google",
  "android_version": "14",
  "sdk": "34",
  "cpu_abi": "arm64-v8a",
  "security_patch": "2024-01-01"
}
```

### 4. Device health check

```bash
dab health --device emulator-5554 --json
```

```json
{
  "device": "emulator-5554",
  "battery": { "level": "85", "status": "2" },
  "storage": { "total_gb": 7.17, "used_gb": 1.82, "free_gb": 5.34, "percent_used": 25.4 },
  "ram": { "total_gb": 1.97, "free_gb": 0.88 },
  "network": { "ip": "10.0.2.15", "ssid": null }
}
```

### 5. Network info

```bash
dab network --device emulator-5554 --json
```

```json
{
  "device": "emulator-5554",
  "ip_addresses": ["10.0.2.15"],
  "ssid": "MyWiFi"
}
```

### 6. App info

```bash
dab app-info --device emulator-5554 --package com.example.myapp --json
```

```json
{
  "package_name": "com.example.myapp",
  "version_code": "42",
  "version_name": "2.1.0"
}
```

Include granted permissions with `--all`:

```bash
dab app-info --device emulator-5554 --package com.example.myapp --all --json
```

```json
{
  "package_name": "com.example.myapp",
  "version_code": "42",
  "version_name": "2.1.0",
  "granted_permissions": [
    "android.permission.CAMERA",
    "android.permission.INTERNET"
  ]
}
```

### 7. Open an app

```bash
dab open --device emulator-5554 --package com.example.myapp --json
```

```json
{ "success": true, "package": "com.example.myapp" }
```

### 8. Uninstall an app

```bash
dab uninstall --device emulator-5554 --package com.example.myapp --json
```

```json
{ "success": true, "package": "com.example.myapp" }
```

### 9. Clear app data

```bash
dab clear --device emulator-5554 --package com.example.myapp --json
```

```json
{ "success": true, "package": "com.example.myapp" }
```

### 10. Force kill an app

```bash
dab force-kill --device emulator-5554 --package com.example.myapp --json
```

```json
{ "success": true, "package": "com.example.myapp" }
```

### 11. Launch a URL or deep link

```bash
dab launch "https://example.com" --device emulator-5554 --json
dab launch "myapp://home" --device emulator-5554 --json
```

```json
{ "success": true, "url": "https://example.com" }
```

### 12. Take a screenshot

```bash
dab screenshot --device emulator-5554 --output /tmp/screen.png --json
```

```json
{ "output": "/tmp/screen.png" }
```

### 13. Install an APK

```bash
dab install /path/to/app.apk --device emulator-5554 --json
```

```json
{ "success": true, "file": "/path/to/app.apk" }
```

### 14. Analyze a local APK (no device needed)

```bash
dab info /path/to/app.apk --json
```

```json
{
  "file": "/path/to/app.apk",
  "package_name": "com.example.myapp",
  "app_name": "My App",
  "version_code": "42",
  "version_name": "2.1.0",
  "permissions": ["android.permission.CAMERA", "android.permission.INTERNET"]
}
```

### 15. Grant permissions

```bash
dab grant --device emulator-5554 \
  --package com.example.myapp \
  --permissions "android.permission.CAMERA,android.permission.RECORD_AUDIO" \
  --json
```

```json
{
  "success": true,
  "package": "com.example.myapp",
  "granted": ["android.permission.CAMERA", "android.permission.RECORD_AUDIO"]
}
```

### 16. Revoke permissions

```bash
dab revoke --device emulator-5554 \
  --package com.example.myapp \
  --permissions "android.permission.CAMERA" \
  --json
```

```json
{
  "success": true,
  "package": "com.example.myapp",
  "revoked": ["android.permission.CAMERA"]
}
```

### 17. Enable ADB over Wi-Fi

```bash
dab wifi --device emulator-5554 --json
```

```json
{ "success": true }
```

### 18. Switch back to USB mode

```bash
dab usb --device emulator-5554 --json
```

```json
{ "success": true }
```

## Error Handling

When `--json` is set, errors are written to **stderr** as JSON:

```json
{ "error": "No connected devices found. Please connect an Android device via USB and enable USB debugging." }
```

Exit codes:
- `0` — success
- `1` — error (check stderr for the JSON error object)

## Common Agent Workflows

### Workflow: Deploy and smoke-test an APK

```bash
# 1. Find the device
DEVICE=$(dab devices --json | jq -r '.devices[0]')

# 2. Install the build
dab install build/app-debug.apk --device "$DEVICE" --json

# 3. Launch the app
PACKAGE="com.example.myapp"
dab open --device "$DEVICE" --package "$PACKAGE" --json

# 4. Take a screenshot for verification
dab screenshot --device "$DEVICE" --output /tmp/smoke.png --json
```

### Workflow: Audit app permissions

```bash
DEVICE=$(dab devices --json | jq -r '.devices[0]')
dab app-info --device "$DEVICE" --package com.example.myapp --all --json \
  | jq '.granted_permissions'
```

### Workflow: Health check before running tests

```bash
DEVICE=$(dab devices --json | jq -r '.devices[0]')
HEALTH=$(dab health --device "$DEVICE" --json)

BATTERY=$(echo "$HEALTH" | jq -r '.battery.level')
echo "Battery: $BATTERY%"

FREE_RAM=$(echo "$HEALTH" | jq -r '.ram.free_gb')
echo "Free RAM: ${FREE_RAM}GB"
```

## Notes for Agents

- Always run `dab devices --json` first to discover available serials.
- Pass `--device` to every subsequent command to avoid ambiguity.
- All structured output is pretty-printed JSON; pipe through `jq` for filtering.
- The `--json` flag suppresses ANSI color codes, making output safe for parsing.
- Environment variable `DAB_DEBUG=1` enables verbose ADB output for debugging.
