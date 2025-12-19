# SH1MMER Custom Payload Examples

This directory contains example custom payloads for SH1MMER that demonstrate how to create your own shims.

## Available Examples

### 1. Hello World (`hello_world/`)

A minimal working example that displays a custom message and provides a simple interactive menu.

**What it demonstrates:**
- Basic payload structure
- System information gathering
- ChromeOS filesystem detection
- Simple text-based menu
- Colorful console output

**Use this example to:**
- Learn the basic structure of a SH1MMER payload
- Understand how the boot process works
- Create simple diagnostic or information display shims
- Build a foundation for more complex payloads

[Read the Hello World README →](hello_world/README.md)

## Prerequisites

Before using these examples, you need:

1. **A raw RMA shim binary** for your Chromebook board
   - Not provided in this repository (see main README for sourcing)
   - Must match your device's board name

2. **Linux or WSL2** with the following packages:
   - `git`
   - `wget`
   - `partx`, `sgdisk`, `mkfs.ext4`, `mkfs.ext2`
   - `tune2fs`, `e2fsck`, `resize2fs`
   - `pv`, `tar`

3. **Root access** to run the wax.sh build script

## Quick Start

### Building an Example

1. Clone this repository:
   ```bash
   git clone https://github.com/MercuryWorkshop/sh1mmer
   cd sh1mmer
   ```

2. Choose an example (e.g., hello_world):
   ```bash
   cd wax
   ```

3. Build the custom shim:
   ```bash
   sudo bash wax.sh -i /path/to/raw_shim.bin --payload_dir ../examples/hello_world
   ```

4. Flash to USB drive:
   - Using Chromebook Recovery Utility (recommended), or
   - Using `dd`: `sudo dd if=/path/to/raw_shim.bin of=/dev/sdX bs=4M status=progress`

### Booting the Custom Shim

1. Insert USB drive into Chromebook
2. Press **ESC + Refresh (↻) + Power (⏻)** to enter recovery mode
3. Press **Ctrl + D** to enable developer mode (if not already enabled)
4. Press **ESC + Refresh + Power** again to boot into recovery
5. Your custom payload should execute!

## Understanding Payload Structure

All payloads must follow this structure:

```
custom_payload/
├── init                              # Entry point script (required)
├── bootstrap/                        # Bootstrap files
│   ├── noarch/                       # Architecture-independent scripts
│   │   ├── your_main_script.sh       # Your main payload logic
│   │   └── other_helpers.sh          # Additional helper scripts
│   ├── x86_64/                       # x86_64-specific binaries (optional)
│   └── aarch64/                      # ARM64-specific binaries (optional)
└── root/                             # Files to overlay on ChromeOS root (optional)
    ├── noarch/                       # Architecture-independent files
    ├── x86_64/                       # x86_64-specific files
    └── aarch64/                      # ARM64-specific files
```

### Key Files

1. **`init`** (Required)
   - First script executed after SH1MMER partition is mounted
   - Sets up environment and launches your main script
   - Should copy busybox and install tools

2. **`bootstrap/noarch/your_script.sh`**
   - Your main payload logic
   - Receives device information as arguments
   - Has access to busybox and any tools you include

3. **`root/` directory** (Optional)
   - Files here get copied over the ChromeOS filesystem in memory
   - Useful for adding persistent scripts or modifying system files
   - Architecture-specific subdirectories for binaries

## Creating Your Own Payload

### Step 1: Create Directory Structure

```bash
mkdir -p my_payload/bootstrap/noarch
```

### Step 2: Create Init Script

**File: `my_payload/init`**

```bash
#!/bootstrap/busybox sh
set -eE

STATEFUL_MNT="$1"

# Detect architecture
ARCHITECTURE="$(uname -m)"
case "$ARCHITECTURE" in
    *x86_64*) ARCHITECTURE=x86_64 ;;
    *aarch64*) ARCHITECTURE=aarch64 ;;
esac

# Setup environment
rm -rf /bin || :
mkdir -p /bin
[ -d "$STATEFUL_MNT/bootstrap/noarch" ] && cp -R "$STATEFUL_MNT/bootstrap/noarch/"* /bin
export PATH=/bin
busybox --install /bin

# Execute your main script
exec /bin/my_main.sh "$@" "$ARCHITECTURE"
```

### Step 3: Create Main Script

**File: `my_payload/bootstrap/noarch/my_main.sh`**

```bash
#!/bin/busybox sh
set -eE

echo "Hello from my custom payload!"
echo "Architecture: $4"
echo "Press Enter for a shell..."
read

exec sh
```

### Step 4: Make Executable

```bash
chmod +x my_payload/init my_payload/bootstrap/noarch/my_main.sh
```

### Step 5: Build and Test

```bash
cd wax
sudo bash wax.sh -i /path/to/raw_shim.bin --payload_dir ../my_payload
```

## Common Use Cases

### Diagnostic Shim

Create a shim that:
- Checks hardware status
- Verifies system health
- Exports logs to USB
- Tests components

### Information Display

Create a shim that:
- Shows ChromeOS version and board
- Displays warranty information
- Shows enrollment status
- Lists installed packages

### Automation Shim

Create a shim that:
- Automatically performs tasks without user interaction
- Configures settings
- Installs software
- Backs up data

### Educational Shim

Create a shim that:
- Demonstrates ChromeOS internals
- Shows the boot process
- Explains partition layout
- Interactive learning tool

## Advanced Topics

### Adding Binaries

To include custom binaries:

1. Place them in `bootstrap/x86_64/` or `bootstrap/aarch64/`
2. Make them executable
3. They'll be available in PATH during boot

### Using Frecon for Graphics

The Beautiful World payload uses `frecon` for graphics. See `sh1mmer_bw/` for examples.

### Mounting ChromeOS Root

To access the ChromeOS filesystem:

```bash
ROOTFS_MNT=/mnt/chromeos
mkdir -p "$ROOTFS_MNT"
mount -o ro /dev/sda3 "$ROOTFS_MNT"  # Adjust device as needed
# Do stuff
umount "$ROOTFS_MNT"
```

### Persistent Changes

To make changes persist across boots, you need to:
1. Mount the stateful partition (usually /dev/sda1)
2. Modify files there
3. Unmount cleanly

**Warning**: This can break ChromeOS if done incorrectly!

## Debugging

### Enable Debug Mode

During boot, press 'x' to enable debug output:
```bash
# In your script
set -x  # Enable command tracing
```

### Drop to Shell Early

During boot, press 's' to drop to a shell before your script runs.

### Add Debug Output

```bash
echo "DEBUG: Got here!" >&2
echo "Variable value: $SOME_VAR" >&2
```

## Best Practices

1. **Always set error handling**: `set -eE` at the start of scripts
2. **Clean up resources**: Unmount filesystems, close files
3. **Provide user feedback**: Show progress, explain what's happening
4. **Handle errors gracefully**: Don't leave the user with a black screen
5. **Test thoroughly**: Try on actual hardware, not just VMs
6. **Document your code**: Future you will thank you

## Limitations

- Cannot modify the kernel (it's signed)
- Changes to ChromeOS root are temporary (in RAM)
- No network access by default (would need drivers/configuration)
- Limited by initramfs environment

## Resources

- **Technical Documentation**: See [EXPLOIT_FLOW.md](../EXPLOIT_FLOW.md) for detailed technical info
- **Full Implementations**: 
  - `../wax/sh1mmer_bw/` - Beautiful World GUI
  - `../wax/sh1mmer_legacy/` - Legacy CLI
- **Main Repository**: https://github.com/MercuryWorkshop/sh1mmer
- **Blog Post**: https://blog.coolelectronics.me/breaking-cros-2/

## Contributing

If you create an interesting example payload:
1. Follow the structure guidelines
2. Include a detailed README
3. Test it on real hardware
4. Submit a pull request!

## Legal and Ethical Considerations

These examples are for **educational and research purposes only**:

- Only use on devices you own or have explicit authorization to modify
- Respect your organization's policies and acceptable use agreements
- Understand that bypassing security measures may violate terms of service
- This is a tool for learning about ChromeOS internals and security research
- The maintainers are not responsible for misuse

## Support

For questions and issues:
- Check the main repository README
- Review the EXPLOIT_FLOW.md technical documentation
- Look at existing implementations (sh1mmer_bw, sh1mmer_legacy)
- Ask in the project's community channels (see main README)

## Credits

SH1MMER was created by the Mercury Workshop team:
- CoolElectronics - Original exploit discovery
- ULTRA BLUE - RootFS verification bypass research
- r58Playz - GUI implementation and initial scripts
- Sharp_Jack - wax build system creation
- OlyB - Current maintainer
- And many other contributors

These examples build upon their excellent work.

---

Happy hacking! Remember to use this knowledge responsibly and ethically.
