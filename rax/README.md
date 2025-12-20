# rax - Rust implementation of wax

**rax** is a Rust reimplementation of the wax shim modifying automation tool for SH1MMER. It provides the same core functionality as wax but written in Rust for improved performance, memory safety, and maintainability.

## Overview

rax is a tool for modifying ChromeOS factory shims to create SH1MMER images. It performs the following operations:

1. Validates and prepares the factory shim image
2. Detects the target architecture (x86_64 or aarch64)
3. Shrinks and optimizes the ROOT partition
4. Creates bootloader and payload partitions
5. Patches in the SH1MMER payload
6. Optimizes and truncates the final image

## Building

To build rax, you need Rust installed (version 1.70 or later recommended):

```bash
cd rax
cargo build --release
```

The compiled binary will be located at `target/release/rax`.

## Usage

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

While rax aims to replicate wax functionality, there are some key differences:

### Advantages of rax:
- **Memory Safety**: Rust's ownership system prevents common bugs like buffer overflows and use-after-free
- **Type Safety**: Strong typing catches errors at compile time
- **Better Error Handling**: Structured error handling with context
- **Maintainability**: More organized code structure with modules
- **Performance**: Compiled binary with optimizations

### Current Limitations:
- Some advanced features may not be fully implemented yet
- This is a work-in-progress reimplementation

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
