# rax - Rust implementation of wax

**rax** is a Rust reimplementation of the wax shim modifying automation tool for SH1MMER. It provides the same core functionality as wax but written in Rust for improved performance, memory safety, and maintainability.

rax can be used both as a **command-line tool** and as a **library** for building custom shims programmatically.

## Overview

rax is a tool for modifying ChromeOS factory shims to create SH1MMER images. It performs the following operations:

1. Validates and prepares the factory shim image
2. Detects the target architecture (x86_64 or aarch64)
3. Shrinks and optimizes the ROOT partition
4. Creates bootloader and payload partitions with cgpt
5. Formats and mounts partitions (ext2 for bootloader, ext4 for payload)
6. Copies payload files, firmware, and optional components
7. Optimizes and truncates the final image

## Building

To build rax, you need Rust installed (version 1.70 or later recommended):

```bash
cd rax
cargo build --release
```

The compiled binary will be located at `target/release/rax`.

## Usage as a Command-Line Tool

Basic usage (equivalent to `wax.sh -i image.bin`):

```bash
sudo ./target/release/rax -i path/to/shim.bin
```

### Command-line Options

```
Options:
  -i, --image <IMAGE>
          Path to factory shim image (required)
  
  -p, --payload <PAYLOAD>
          Main payload ('bw' or 'legacy') [default: bw]
  
      --payload-dir <PAYLOAD_DIR>
          Custom main payload directory
  
  -s, --sh1mmer-part-size <SH1MMER_PART_SIZE>
          Partition size for payload(s) [default: 72M]
  
  -e, --extra-payload-dir <EXTRA_PAYLOAD_DIR>
          Extra payload directory
  
      --firmware-dir <FIRMWARE_DIR>
          Insert firmware from directory
  
  -m, --mounted-payload-dir <MOUNTED_PAYLOAD_DIR>
          Mounted payload directory
  
      --chromebrew <CHROMEBREW>
          Chromebrew payload (mounted)
  
      --bootloader-dir <BOOTLOADER_DIR>
          Path to bootloader data
  
      --bootloader-part-size <BOOTLOADER_PART_SIZE>
          Bootloader rootfs partition size [default: 4M]
  
      --arch <ARCH>
          Force architecture (x86_64 or aarch64)
  
  -d, --debug
          Print debug messages
  
      --fast
          Fast/dirty build, larger image size
  
      --finalsizefile <FINALSIZEFILE>
          Write final image size in bytes to this file
```

### Examples

Create a SH1MMER image with default settings:
```bash
sudo rax -i shimhatch.bin
```

Use a custom payload and enable debug mode:
```bash
sudo rax -i shimhatch.bin -p legacy -d
```

Specify custom partition sizes:
```bash
sudo rax -i shimhatch.bin -s 100M --bootloader-part-size 8M
```

Fast mode (skip optimization):
```bash
sudo rax -i shimhatch.bin --fast
```

## Usage as a Library

rax can be used as a Rust library to build custom shims programmatically. Add it to your `Cargo.toml`:

```toml
[dependencies]
rax = { path = "../rax" }
anyhow = "1.0"
```

### Example: Building a custom shim

```rust
use rax::{ShimConfig, WaxOperations};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // Create a custom configuration
    let config = ShimConfig {
        image: PathBuf::from("my_shim.bin"),
        bootloader_dir: PathBuf::from("wax/bootstrap"),
        payload_dir: PathBuf::from("my_custom_payload"),
        extra_payload_dir: None,
        firmware_dir: Some(PathBuf::from("custom_firmware")),
        mounted_payload_dir: None,
        chromebrew: None,
        sh1mmer_part_size: 72 * 1024 * 1024,  // 72MB
        bootloader_part_size: 4 * 1024 * 1024, // 4MB
        target_arch: Some("x86_64".to_string()),
        fast: false,
        finalsizefile: None,
    };

    // Build the shim
    let mut ops = WaxOperations::with_config(config)?;
    ops.execute()?;

    println!("Custom shim built successfully!");
    Ok(())
}
```

### Example: Using with custom payload directory

```rust
use rax::{ShimConfig, WaxOperations};
use std::path::PathBuf;

fn build_shim_with_custom_payload(
    image_path: &str,
    payload_path: &str,
) -> anyhow::Result<()> {
    let config = ShimConfig {
        image: PathBuf::from(image_path),
        bootloader_dir: PathBuf::from("wax/bootstrap"),
        payload_dir: PathBuf::from(payload_path),  // Your custom payload!
        extra_payload_dir: None,
        firmware_dir: None,
        mounted_payload_dir: None,
        chromebrew: None,
        sh1mmer_part_size: 100 * 1024 * 1024,  // 100MB for larger payload
        bootloader_part_size: 4 * 1024 * 1024,
        target_arch: None,  // Auto-detect
        fast: false,
        finalsizefile: Some(PathBuf::from("final_size.txt")),
    };

    let mut ops = WaxOperations::with_config(config)?;
    ops.execute()?;

    Ok(())
}
```

This modular design allows you to:
- **Build custom payloads** and integrate them into shims
- **Automate shim generation** in larger build systems
- **Create specialized shim variants** for different use cases
- **Integrate with testing frameworks** for validation

## Requirements

rax requires the following system utilities to be installed:

- `partx` - Partition table manipulation
- `sgdisk` - GPT partition editor
- `mkfs.ext4` - ext4 filesystem creation
- `mkfs.ext2` - ext2 filesystem creation
- `tune2fs` - ext2/3/4 filesystem tuning
- `e2fsck` - ext2/3/4 filesystem checking
- `resize2fs` - ext2/3/4 filesystem resizing
- `file` - File type detection
- `numfmt` - Number formatting
- `pv` - Progress viewer
- `tar` - Archive utility

These are typically available in the following packages:
- Debian/Ubuntu: `util-linux`, `gdisk`, `e2fsprogs`, `file`, `coreutils`, `pv`, `tar`
- Arch Linux: `util-linux`, `gptfdisk`, `e2fsprogs`, `file`, `coreutils`, `pv`, `tar`
- Fedora/RHEL: `util-linux`, `gdisk`, `e2fsprogs`, `file`, `coreutils`, `pv`, `tar`

## Differences from wax

rax provides the same functionality as wax with these improvements:

### Advantages of rax:
- **Memory Safety**: Rust's ownership system prevents common bugs like buffer overflows and use-after-free
- **Type Safety**: Strong typing catches errors at compile time
- **Better Error Handling**: Structured error handling with context
- **Maintainability**: More organized code structure with modules
- **Performance**: Compiled binary with optimizations
- **Library Support**: Can be used as a library for custom shim building
- **Modular Design**: Public API allows programmatic shim creation

### Implementation Status:
- ✅ **Fully Implemented**: All core wax features
  - GPT partition manipulation with cgpt
  - Bootloader partition creation and patching
  - SH1MMER payload partition creation and patching
  - Architecture detection (x86_64/aarch64)
  - ROOT partition shrinking and optimization
  - Partition squashing
  - Image truncation
  - Chromebrew extraction
  - Extra payloads, firmware, and mounted payloads support
  
### API Differences:
- Command-line arguments use `--` for long options (e.g., `--debug` instead of `-d` alone)
- More descriptive error messages with context
- Progress output uses colored terminal output

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Output

```bash
cargo run -- -i path/to/image.bin -d
```

### Code Structure

- `src/main.rs` - Entry point and dependency checking
- `src/cli.rs` - Command-line argument parsing
- `src/common.rs` - Common utilities (logging, byte parsing)
- `src/operations.rs` - Core image modification operations

## License

This project follows the same license as the parent SH1MMER project.

## Credits

- Original wax implementation: CoolElectronics, Sharp_Jack, r58playz, Rafflesia, OlyB
- Rust reimplementation (rax): Part of the SH1MMER project

## Contributing

Contributions are welcome! Please ensure:
1. Code compiles without warnings: `cargo build --release`
2. All tests pass: `cargo test`
3. Code is formatted: `cargo fmt`
4. No clippy warnings: `cargo clippy`
