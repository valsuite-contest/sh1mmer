# MinShim - Minimal Bootable Shim Builder

A Rust-based tool for building minimal bootable shims from RMA images. Creates ultra-small shims that boot directly to a custom payload.

## Features

- **Minimal size**: Creates shims as small as possible (shrinks root partition to minimum)
- **Simple payload**: Boots to a screen displaying your custom message
- **No dependencies on existing build tools**: Standalone Rust implementation
- **Clean code**: Well-structured, readable Rust code
- **Fast**: Efficient partition manipulation and filesystem operations

## Requirements

### System Tools
- `losetup` - Loop device management
- `sfdisk` - Partition table manipulation
- `sgdisk` - GPT partition table operations
- `e2fsck` - Filesystem checking
- `resize2fs` - Ext2/3/4 filesystem resizing
- `tune2fs` - Filesystem parameter tuning
- `mkfs.ext4` - Ext4 filesystem creation
- `cgpt` - ChromeOS GPT tool (from wax/lib/bin/ or system)

### Build Requirements
- Rust 1.70 or later
- Cargo

## Installation

```bash
cd minshim
cargo build --release
```

The binary will be available at `target/release/minshim`.

## Usage

### Basic Usage

```bash
sudo ./target/release/minshim -i /path/to/raw_shim.bin
```

This will:
1. Copy the input image
2. Shrink the root partition to minimum size
3. Delete the state partition
4. Create a minimal payload partition (10MB by default)
5. Add a payload that displays "payload ran" and sleeps
6. Truncate the image to final size

Output: `raw_shim.minshim.bin`

### Custom Output Path

```bash
sudo ./target/release/minshim -i input.bin -o custom_output.bin
```

### Custom Payload Message

```bash
sudo ./target/release/minshim -i input.bin -m "Hello from MinShim!"
```

### Custom Payload Size

```bash
sudo ./target/release/minshim -i input.bin -s 20
```

This creates a 20MB payload partition (default is 10MB).

### Full Example

```bash
sudo ./target/release/minshim \
    -i raw_shim.bin \
    -o my_minshim.bin \
    -m "payload ran" \
    -s 15
```

## How It Works

### 1. Partition Manipulation

MinShim performs the following operations on the RMA shim:

1. **Shrinks ROOT-A partition (p3)**:
   - Enables RW mount by flipping the read-only bit at offset 0x464+3
   - Runs e2fsck to check filesystem integrity
   - Runs resize2fs to shrink to minimum size
   - Disables RW mount

2. **Deletes STATE partition (p1)**:
   - Uses sfdisk to remove partition 1
   - Frees space for the payload partition

3. **Creates PAYLOAD partition (new p1)**:
   - Uses cgpt to add partition at the end of the disk
   - Formats as ext4
   - Labels as "PAYLOAD"

### 2. Payload Creation

The payload is a minimal shell script that:
- Displays a bordered message box
- Shows your custom message
- Sleeps indefinitely (keeps the screen visible)

Required structure:
```
payload_partition/
├── dev_image/
│   ├── etc/
│   │   └── lsb-factory      # Required for boot detection
│   └── factory/
│       └── sh/
└── init                      # Main payload script (executable)
```

### 3. Boot Process

When the Chromebook boots from the modified shim:

1. Firmware verifies and loads KERNEL-A (p2) - unchanged, signed
2. Kernel mounts ROOT-A (p3) - shrunk but functional
3. Factory scripts detect the payload partition via lsb-factory
4. The `/init` script in the payload partition executes
5. Your custom message appears on screen

## Architecture

```
minshim/
├── Cargo.toml              # Dependencies and metadata
├── README.md               # This file
└── src/
    ├── main.rs             # Main program logic
    ├── partition.rs        # Partition table operations
    ├── filesystem.rs       # Filesystem operations
    └── payload.rs          # Payload generation
```

### Key Modules

- **main.rs**: Orchestrates the build process
  - Command-line argument parsing
  - Loop device management
  - Cleanup handling

- **partition.rs**: Partition table reading and manipulation
  - Reads existing partition layout
  - Provides partition information

- **filesystem.rs**: Ext2/4 filesystem operations
  - Filesystem creation
  - Size calculations

- **payload.rs**: Payload generation
  - Creates init script
  - Sets up required directory structure

## Comparison to Traditional Build

### Traditional (wax.sh)
- Size: ~1GB+ (includes full Beautiful World GUI, tools, etc.)
- Dependencies: bash, wax libraries, multiple scripts
- Complexity: ~1000+ lines across multiple files
- Boot time: Slower (loads full GUI)

### MinShim
- Size: ~50-100MB (minimal root + small payload)
- Dependencies: Standalone Rust binary
- Complexity: ~400 lines of clean Rust
- Boot time: Fast (minimal init)

## Output Example

```
╔══════════════════════════════════════════╗
║          MinShim Builder v0.1            ║
║   Minimal Bootable Shim Generator        ║
╚══════════════════════════════════════════╝

Input:  raw_shim.bin
Output: raw_shim.minshim.bin

[1/7] Copying input image...
[2/7] Setting up loop device...
      Loop device: /dev/loop0
[3/7] Analyzing partition table...
      Found 3 partitions
      - Partition 1: partition 1
      - Partition 2: partition 2
      - Partition 3: partition 3
[4/7] Shrinking root partition...
      Enabling RW mount on root partition...
      Checking filesystem...
      Resizing to minimum size...
      New size: 45 MB
[5/7] Deleting state partition...
[6/7] Creating payload partition...
      Creating partition at sector 123456 with 20480 sectors
      Formatting /dev/loop0p1...
[7/7] Finalizing image...
      Truncating to 60 MB

✓ Success! Minimal shim created: raw_shim.minshim.bin
  Flash with: sudo dd if=raw_shim.minshim.bin of=/dev/sdX bs=4M status=progress
```

## Troubleshooting

### "cgpt not found"
Ensure cgpt is available. Either:
- Use from wax: `cp ../wax/lib/bin/x86_64/cgpt /usr/local/bin/`
- Or compile from vboot_reference

### "This tool requires root privileges"
Run with sudo:
```bash
sudo ./target/release/minshim -i input.bin
```

### "Failed to detach loop device"
The cleanup guard should handle this, but if it fails:
```bash
sudo losetup -D  # Detach all loop devices
```

### "mkfs.ext4 failed"
Ensure e2fsprogs is installed:
```bash
sudo apt-get install e2fsprogs  # Debian/Ubuntu
sudo yum install e2fsprogs      # RHEL/CentOS
```

## Technical Details

### Read-Only Bit Manipulation

The ext2/ext4 read-only feature bit is at byte offset `0x464 + 3` in the superblock:
- `0xFF` = Read-only enabled
- `0x00` = Read-only disabled

This is a simple filesystem flag, not cryptographic protection.

### Partition Numbering

After modification:
- Partition 1: PAYLOAD (new, ext4, 10MB default)
- Partition 2: KERNEL-A (unchanged, signed)
- Partition 3: ROOT-A (shrunk, ext2)

### Safety Margins

- 35 sectors added after the last partition for GPT backup table
- Filesystem resize includes safety checks (e2fsck before resize)
- GPT table is fixed after truncation (sgdisk -e)

## Development

### Building from Source

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Code Style

Format code with:
```bash
cargo fmt
```

Check for issues:
```bash
cargo clippy
```

## License

This tool follows the same license as the parent repository.

## Credits

Built as a minimal, standalone alternative to the wax build system. Designed for users who want:
- Maximum control over the build process
- Minimal shim sizes
- Clean, understandable code
- No bash script dependencies

## Security Notice

This tool is for **educational and research purposes only**:
- Only use on devices you own
- Understand the legal implications
- Respect organizational policies
- This modifies disk images - use with caution

## Contributing

Contributions welcome! Please ensure:
- Code follows Rust conventions
- Tests pass
- Documentation is updated
- Commit messages are clear

---

**Remember**: With great power comes great responsibility. Use ethically.
