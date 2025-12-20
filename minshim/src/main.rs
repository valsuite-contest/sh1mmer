use anyhow::{Context, Result};
use clap::Parser;
use std::fs::{self, File};
use std::io::{Write, Seek, SeekFrom};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

mod partition;

use partition::PartitionTable;

#[derive(Parser, Debug)]
#[command(name = "minshim")]
#[command(about = "Build minimal bootable shims from RMA images", long_about = None)]
struct Args {
    /// Path to the input RMA shim image
    #[arg(short, long)]
    input: PathBuf,

    /// Path to the output modified shim image
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Size of the payload partition in MB (default: 10)
    #[arg(short = 's', long, default_value = "10")]
    payload_size: u64,

    /// Custom payload message (default: "payload ran")
    #[arg(short = 'm', long, default_value = "payload ran")]
    message: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("╔══════════════════════════════════════════╗");
    println!("║          MinShim Builder v0.1            ║");
    println!("║   Minimal Bootable Shim Generator        ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    // Determine output path
    let output = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("minshim.bin");
        path
    });

    // Check root permissions
    if !nix::unistd::Uid::effective().is_root() {
        anyhow::bail!("This tool requires root privileges. Please run with sudo.");
    }

    println!("Input:  {}", args.input.display());
    println!("Output: {}", output.display());
    println!();

    // Copy input to output if different
    if args.input != output {
        println!("[1/7] Copying input image...");
        fs::copy(&args.input, &output)
            .context("Failed to copy input image")?;
    }

    println!("[2/7] Setting up loop device...");
    let loop_device = setup_loop_device(&output)?;
    println!("      Loop device: {}", loop_device);

    // Ensure cleanup on exit
    let _cleanup = CleanupGuard::new(loop_device.clone());

    println!("[3/7] Analyzing partition table...");
    let pt = PartitionTable::read(&loop_device)?;
    pt.print_summary();

    println!("[4/7] Shrinking root partition...");
    shrink_root_partition(&loop_device)?;

    println!("[5/7] Deleting state partition...");
    delete_partition(&output, 1)?;
    
    // Reload partition table
    reload_partitions(&loop_device)?;

    println!("[6/7] Creating payload partition...");
    let payload_mb = args.payload_size;
    create_payload_partition(&loop_device, &output, payload_mb, &args.message)?;

    println!("[7/7] Finalizing image...");
    cleanup_loop_device(&loop_device)?;
    truncate_image(&output)?;

    println!();
    println!("✓ Success! Minimal shim created: {}", output.display());
    println!("  Flash with: sudo dd if={} of=/dev/sdX bs=4M status=progress", output.display());
    println!();

    Ok(())
}

struct CleanupGuard {
    loop_device: String,
}

impl CleanupGuard {
    fn new(loop_device: String) -> Self {
        CleanupGuard { loop_device }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = cleanup_loop_device(&self.loop_device);
    }
}

fn setup_loop_device(image_path: &Path) -> Result<String> {
    let output = Command::new("losetup")
        .arg("-f")
        .arg("--show")
        .arg("-P")
        .arg(image_path)
        .output()
        .context("Failed to run losetup")?;

    if !output.status.success() {
        anyhow::bail!("losetup failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn cleanup_loop_device(loop_device: &str) -> Result<()> {
    let output = Command::new("losetup")
        .arg("-d")
        .arg(loop_device)
        .output()
        .context("Failed to detach loop device")?;

    if !output.status.success() {
        eprintln!("Warning: Failed to detach loop device: {}", 
                  String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

fn reload_partitions(loop_device: &str) -> Result<()> {
    Command::new("partx")
        .arg("-u")
        .arg(loop_device)
        .output()
        .context("Failed to reload partitions")?;
    
    std::thread::sleep(std::time::Duration::from_millis(500));
    Ok(())
}

fn shrink_root_partition(loop_device: &str) -> Result<()> {
    let root_part = format!("{}p3", loop_device);
    
    // Enable RW mount
    println!("      Enabling RW mount on root partition...");
    enable_rw_mount(&root_part)?;

    // Check filesystem
    println!("      Checking filesystem...");
    let output = Command::new("e2fsck")
        .arg("-fy")
        .arg(&root_part)
        .output()
        .context("Failed to run e2fsck")?;
    
    if !output.status.success() && output.status.code() != Some(1) {
        eprintln!("e2fsck warning: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Resize to minimum
    println!("      Resizing to minimum size...");
    let output = Command::new("resize2fs")
        .arg("-M")
        .arg(&root_part)
        .output()
        .context("Failed to run resize2fs")?;
    
    if !output.status.success() {
        anyhow::bail!("resize2fs failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Get new size
    let output = Command::new("tune2fs")
        .arg("-l")
        .arg(&root_part)
        .output()
        .context("Failed to run tune2fs")?;

    let info = String::from_utf8_lossy(&output.stdout);
    let block_size: u64 = info.lines()
        .find(|l| l.contains("Block size"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|s| s.parse().ok())
        .context("Failed to get block size")?;
    
    let block_count: u64 = info.lines()
        .find(|l| l.contains("Block count"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|s| s.parse().ok())
        .context("Failed to get block count")?;

    let new_size = block_size * block_count;
    println!("      New size: {} MB", new_size / 1024 / 1024);

    // Disable RW mount
    disable_rw_mount(&root_part)?;

    Ok(())
}

fn enable_rw_mount(partition: &str) -> Result<()> {
    let ro_offset = 0x464 + 3;
    let mut file = File::options().write(true).open(partition)?;
    file.seek(SeekFrom::Start(ro_offset))?;
    file.write_all(&[0x00])?;
    Ok(())
}

fn disable_rw_mount(partition: &str) -> Result<()> {
    let ro_offset = 0x464 + 3;
    let mut file = File::options().write(true).open(partition)?;
    file.seek(SeekFrom::Start(ro_offset))?;
    file.write_all(&[0xFF])?;
    Ok(())
}

fn delete_partition(image_path: &Path, partition_num: u32) -> Result<()> {
    let output = Command::new("sfdisk")
        .arg("--delete")
        .arg(image_path)
        .arg(partition_num.to_string())
        .output()
        .context("Failed to delete partition")?;

    if !output.status.success() {
        eprintln!("sfdisk warning: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

fn create_payload_partition(loop_device: &str, image_path: &Path, size_mb: u64, message: &str) -> Result<()> {
    // Get sector size
    let output = Command::new("sfdisk")
        .arg("-l")
        .arg(image_path)
        .output()
        .context("Failed to get sector size")?;
    
    let info = String::from_utf8_lossy(&output.stdout);
    let sector_size: u64 = info.lines()
        .find(|l| l.contains("Sector size"))
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|s| s.parse().ok())
        .unwrap_or(512);

    // Find cgpt tool
    let cgpt = find_cgpt()?;

    // Get last sector
    let output = Command::new("sfdisk")
        .arg("-l")
        .arg("-o")
        .arg("end")
        .arg(image_path)
        .output()
        .context("Failed to get last sector")?;

    let last_sector: u64 = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .max()
        .unwrap_or(0);

    let payload_sectors = (size_mb * 1024 * 1024) / sector_size;
    let start_sector = last_sector + 1;

    println!("      Creating partition at sector {} with {} sectors", start_sector, payload_sectors);

    // Add partition using cgpt
    let output = Command::new(&cgpt)
        .arg("add")
        .arg("-i")
        .arg("1")
        .arg("-b")
        .arg(start_sector.to_string())
        .arg("-s")
        .arg(payload_sectors.to_string())
        .arg("-t")
        .arg("data")
        .arg("-l")
        .arg("PAYLOAD")
        .arg(loop_device)
        .output()
        .context("Failed to add partition")?;

    if !output.status.success() {
        anyhow::bail!("cgpt add failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Reload partitions
    reload_partitions(loop_device)?;

    // Format partition
    let part1 = format!("{}p1", loop_device);
    println!("      Formatting {}...", part1);
    
    let output = Command::new("mkfs.ext4")
        .arg("-F")
        .arg("-L")
        .arg("PAYLOAD")
        .arg(&part1)
        .output()
        .context("Failed to format partition")?;

    if !output.status.success() {
        anyhow::bail!("mkfs.ext4 failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Mount and populate
    let mount_point = std::env::temp_dir().join(format!("minshim_{}", std::process::id()));
    fs::create_dir_all(&mount_point)?;
    
    let output = Command::new("mount")
        .arg(&part1)
        .arg(&mount_point)
        .output()
        .context("Failed to mount partition")?;

    if !output.status.success() {
        anyhow::bail!("mount failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Create payload structure
    create_payload_files(&mount_point, message)?;

    // Unmount
    Command::new("umount")
        .arg(&mount_point)
        .output()
        .context("Failed to unmount")?;

    fs::remove_dir(&mount_point)?;

    Ok(())
}

fn create_payload_files(mount_point: &Path, message: &str) -> Result<()> {
    // Create directory structure
    fs::create_dir_all(mount_point.join("dev_image/etc"))?;
    fs::create_dir_all(mount_point.join("dev_image/factory/sh"))?;
    
    // Create lsb-factory (required for boot)
    File::create(mount_point.join("dev_image/etc/lsb-factory"))?;

    // Create init script
    let init_content = format!(r#"#!/bin/sh
clear
echo "╔══════════════════════════════════════════╗"
echo "║         Minimal Boot Payload             ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "{}"
echo ""
echo "Sleeping indefinitely..."
sleep infinity
"#, message);

    let init_path = mount_point.join("init");
    fs::write(&init_path, init_content)?;
    
    // Set executable permissions
    let metadata = fs::metadata(&init_path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&init_path, permissions)?;

    Ok(())
}

fn find_cgpt() -> Result<PathBuf> {
    // Try to find cgpt in wax directory
    let possible_paths = [
        "wax/lib/bin/x86_64/cgpt",
        "wax/lib/bin/aarch64/cgpt",
        "/usr/local/bin/cgpt",
        "/usr/bin/cgpt",
    ];

    for path in &possible_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    anyhow::bail!("cgpt not found. Please ensure it's available in wax/lib/bin/ or in PATH")
}

fn truncate_image(image_path: &Path) -> Result<()> {
    // Get final sector
    let output = Command::new("sfdisk")
        .arg("-l")
        .arg("-o")
        .arg("end")
        .arg(image_path)
        .output()
        .context("Failed to get sectors")?;

    let info = String::from_utf8_lossy(&output.stdout);
    let last_sector: u64 = info.lines()
        .filter_map(|l| l.trim().parse().ok())
        .max()
        .unwrap_or(0);

    // Get sector size
    let output = Command::new("sfdisk")
        .arg("-l")
        .arg(image_path)
        .output()
        .context("Failed to get sector info")?;
    
    let info = String::from_utf8_lossy(&output.stdout);
    let sector_size: u64 = info.lines()
        .find(|l| l.contains("Sector size"))
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|s| s.parse().ok())
        .unwrap_or(512);

    let buffer = 35; // Safety margin
    let final_size = (last_sector + buffer) * sector_size;

    println!("      Truncating to {} MB", final_size / 1024 / 1024);

    let file = File::options().write(true).open(image_path)?;
    file.set_len(final_size)?;

    // Fix GPT backup table
    let output = Command::new("sgdisk")
        .arg("-e")
        .arg(image_path)
        .output()
        .context("Failed to fix GPT")?;

    if !output.status.success() {
        eprintln!("sgdisk warning: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}
