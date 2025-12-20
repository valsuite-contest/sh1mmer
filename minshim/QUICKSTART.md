# MinShim Quick Start

## Build the Tool

```bash
cd minshim
cargo build --release
```

## Run MinShim

```bash
# Basic usage - creates a shim that displays "payload ran"
sudo ./target/release/minshim -i /path/to/raw_shim.bin

# Custom message
sudo ./target/release/minshim -i raw_shim.bin -m "payload ran"

# Custom output path
sudo ./target/release/minshim -i raw_shim.bin -o my_shim.bin

# Larger payload partition (20MB instead of 10MB default)
sudo ./target/release/minshim -i raw_shim.bin -s 20
```

## Flash to USB

```bash
sudo dd if=raw_shim.minshim.bin of=/dev/sdX bs=4M status=progress
```

Replace `/dev/sdX` with your USB device (check with `lsblk`).

## Boot

1. Insert USB into Chromebook
2. Press ESC + Refresh + Power to enter recovery
3. Press Ctrl + D to enable developer mode (if needed)
4. Press ESC + Refresh + Power again to boot from USB
5. You should see the payload message screen

## What You Get

A minimal bootable shim that:
- Boots to a screen displaying your message
- Sleeps indefinitely on that screen
- Is as small as possible (~50-100MB vs 1GB+ for full builds)

## Example Output

```
╔══════════════════════════════════════════╗
║         Minimal Boot Payload             ║
╚══════════════════════════════════════════╝

payload ran

Sleeping indefinitely...
```

The screen will stay at this message until you power off the Chromebook.
