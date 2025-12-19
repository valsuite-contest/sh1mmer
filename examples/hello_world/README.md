# Hello World Custom SH1MMER Example

This directory contains a minimal working example of a custom SH1MMER payload that displays a "Hello World" message and provides a simple menu.

## What This Example Does

1. Displays a colorful welcome message
2. Shows system information (architecture, kernel, hardware)
3. Attempts to identify the ChromeOS installation
4. Provides a simple menu with options to:
   - Drop to a busybox shell
   - View partition layout
   - Reboot

## Directory Structure

```
hello_world/
├── README.md                          # This file
├── init                               # Main init script (entry point)
└── bootstrap/
    └── noarch/
        └── hello_world.sh             # The actual payload script
```

## How to Build

1. Make sure you have a raw RMA shim binary for your Chromebook board
2. Make the scripts executable:
   ```bash
   chmod +x init
   chmod +x bootstrap/noarch/hello_world.sh
   ```

3. Build the custom shim using wax:
   ```bash
   cd ../../wax
   sudo bash wax.sh -i /path/to/raw_shim.bin --payload_dir ../examples/hello_world
   ```

4. Flash the resulting bin file to a USB drive using:
   - Chromebook Recovery Utility, or
   - `dd` on Linux: `sudo dd if=raw_shim.bin of=/dev/sdX bs=4M status=progress`

## How to Use

1. Insert the USB drive into your Chromebook
2. Boot into recovery mode: Press **ESC + Refresh + Power** simultaneously
3. When prompted, press **Ctrl + D** to enable developer mode (or skip if already enabled)
4. Press **ESC + Refresh + Power** again to boot into recovery
5. The Chromebook should boot from the USB and show the Hello World message

## Understanding the Code

### `init` Script

This is the entry point that:
- Detects the system architecture (x86_64 or aarch64)
- Copies busybox and scripts from the SH1MMER partition
- Executes the main hello_world.sh script

### `hello_world.sh` Script

This is the main payload that:
- Clears the screen and displays a fancy message
- Gathers and displays system information
- Attempts to mount and read the ChromeOS root filesystem
- Provides an interactive menu

## Customization Ideas

You can extend this example to:

1. **Add more menu options**:
   - Check if device is enrolled
   - Display hardware information
   - Test network connectivity
   - Run diagnostic commands

2. **Add graphics**:
   - Use `frecon` for framebuffer graphics
   - Display images or custom UI

3. **Add automation**:
   - Automatically perform actions without user input
   - Log information to a file
   - Execute a sequence of commands

4. **Add tools**:
   - Copy additional binaries to the payload
   - Include diagnostic utilities
   - Add networking tools

## Example Modifications

### Add a System Health Check

Add this function to `hello_world.sh`:

```bash
check_system_health() {
    echo "=== System Health Check ==="
    echo "Disk Usage:"
    df -h 2>/dev/null | head -n 5
    echo ""
    echo "Memory Usage:"
    free -h
    echo ""
    echo "Load Average:"
    uptime
    echo ""
}
```

Then add it to the menu.

### Display ASCII Art

Add a custom ASCII art banner:

```bash
cat << "EOF"
   _   _      _ _        __        __         _     _ 
  | | | | ___| | | ___   \ \      / /__  _ __| | __| |
  | |_| |/ _ \ | |/ _ \   \ \ /\ / / _ \| '__| |/ _` |
  |  _  |  __/ | | (_) |   \ V  V / (_) | |  | | (_| |
  |_| |_|\___|_|_|\___/     \_/\_/ \___/|_|  |_|\__,_|
                                                        
EOF
```

### Add Color Themes

Define different color schemes:

```bash
# Dark theme
THEME_BG="\033[40m"
THEME_FG="\033[97m"

# Light theme
# THEME_BG="\033[107m"
# THEME_FG="\033[30m"
```

## Notes

- This example runs entirely in the initramfs/early boot environment
- It does **not** patch or modify the ChromeOS installation
- All changes are temporary and lost on reboot
- For persistence, you would need to modify the actual ChromeOS root filesystem

## Next Steps

After understanding this example, check out:
- `../sh1mmer_bw/` - Full Beautiful World GUI implementation
- `../sh1mmer_legacy/` - Full Legacy CLI implementation
- `../../EXPLOIT_FLOW.md` - Detailed technical documentation

## Troubleshooting

**Script doesn't execute:**
- Make sure scripts are executable: `chmod +x init bootstrap/noarch/hello_world.sh`
- Check that the payload_dir path is correct when running wax.sh

**Black screen after booting:**
- The device might not support frecon graphics
- Try adding debug output earlier in the script
- Use the 's' key during boot to drop to a debug shell

**Device won't boot from USB:**
- Ensure developer mode is enabled
- Check that the USB drive is properly flashed
- Try a different USB port
- Verify the raw shim was for the correct board

## Security Considerations

This is for **educational purposes** only:
- Only use on devices you own
- Respect your organization's policies
- Understand the legal implications
- This is a research and learning tool

## Credits

This example is based on the SH1MMER project by Mercury Workshop:
- CoolElectronics - Original exploit
- r58Playz - GUI implementation
- Sharp_Jack - wax build system
- OlyB - Maintainer

See the main repository for full credits: https://github.com/MercuryWorkshop/sh1mmer
