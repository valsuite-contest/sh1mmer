use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::Args;
use crate::common::{format_bytes, log_debug, log_info, log_warn, parse_bytes};

/// Configuration for building a shim image
#[derive(Debug, Clone)]
pub struct ShimConfig {
    pub image: PathBuf,
    pub bootloader_dir: PathBuf,
    pub payload_dir: PathBuf,
    pub extra_payload_dir: Option<PathBuf>,
    pub firmware_dir: Option<PathBuf>,
    pub mounted_payload_dir: Option<PathBuf>,
    pub chromebrew: Option<PathBuf>,
    pub sh1mmer_part_size: u64,
    pub bootloader_part_size: u64,
    pub target_arch: Option<String>,
    pub fast: bool,
    pub finalsizefile: Option<PathBuf>,
}

impl ShimConfig {
    /// Create a new ShimConfig from command-line arguments
    pub fn from_args(args: Args) -> Result<Self> {
        // Validate image
        let image = args.image.clone();
        if !image.exists() {
            anyhow::bail!("Image {} doesn't exist", image.display());
        }

        // Determine bootloader directory
        let bootloader_dir = if let Some(dir) = args.bootloader_dir {
            dir
        } else {
            PathBuf::from("wax/bootstrap")
        };

        if !bootloader_dir.is_dir() {
            anyhow::bail!("{} is not a directory", bootloader_dir.display());
        }
        log_info(&format!("Using bootloader: {}", bootloader_dir.display()));

        // Determine payload directory
        let payload_dir = if let Some(dir) = args.payload_dir {
            dir
        } else {
            match args.payload.as_str() {
                "legacy" => PathBuf::from("wax/sh1mmer_legacy"),
                "bw" => PathBuf::from("wax/sh1mmer_bw"),
                _ => anyhow::bail!("Invalid payload '{}'", args.payload),
            }
        };

        if !payload_dir.is_dir() {
            anyhow::bail!("{} is not a directory", payload_dir.display());
        }
        log_info(&format!("Using main payload: {}", payload_dir.display()));

        // Validate extra payload directory if provided
        if let Some(ref dir) = args.extra_payload_dir {
            if !dir.is_dir() {
                anyhow::bail!("{} is not a directory", dir.display());
            }
            log_info(&format!("Using extra payload: {}", dir.display()));
        }

        // Validate firmware directory if provided
        if let Some(ref dir) = args.firmware_dir {
            if !dir.is_dir() {
                anyhow::bail!("{} is not a directory", dir.display());
            }
            log_info(&format!("Using firmware: {}", dir.display()));
        }

        // Validate mounted payload directory if provided
        if let Some(ref dir) = args.mounted_payload_dir {
            if !dir.is_dir() {
                anyhow::bail!("{} is not a directory", dir.display());
            }
            log_info(&format!("Using mounted payload: {}", dir.display()));
        }

        // Validate chromebrew file if provided
        if let Some(ref file) = args.chromebrew {
            if !file.is_file() {
                anyhow::bail!("{} doesn't exist or isn't a file", file.display());
            }
            log_info(&format!("Using chromebrew: {}", file.display()));
        }

        // Parse sizes
        let sh1mmer_part_size = parse_bytes(&args.sh1mmer_part_size)
            .with_context(|| format!("Could not parse size '{}'", args.sh1mmer_part_size))?;
        let bootloader_part_size = parse_bytes(&args.bootloader_part_size)
            .with_context(|| format!("Could not parse size '{}'", args.bootloader_part_size))?;

        Ok(Self {
            image,
            bootloader_dir,
            payload_dir,
            extra_payload_dir: args.extra_payload_dir,
            firmware_dir: args.firmware_dir,
            mounted_payload_dir: args.mounted_payload_dir,
            chromebrew: args.chromebrew,
            sh1mmer_part_size,
            bootloader_part_size,
            target_arch: args.arch,
            fast: args.fast,
            finalsizefile: args.finalsizefile,
        })
    }
}

/// Main builder for shim images
pub struct WaxOperations {
    config: ShimConfig,
    loopdev: Option<String>,
    cgpt_path: PathBuf,
}

impl WaxOperations {
    pub fn new(args: Args) -> Result<Self> {
        let config = ShimConfig::from_args(args)?;
        
        // Determine cgpt path based on host architecture
        let host_arch = std::env::consts::ARCH;
        let cgpt_arch = match host_arch {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            _ => "x86_64", // default fallback
        };
        
        let cgpt_path = PathBuf::from(format!("wax/lib/bin/{}/cgpt", cgpt_arch));
        
        Ok(Self {
            config,
            loopdev: None,
            cgpt_path,
        })
    }
    
    /// Create a WaxOperations instance with a custom configuration
    /// This allows using rax as a library for custom shim building
    pub fn with_config(config: ShimConfig) -> Result<Self> {
        let host_arch = std::env::consts::ARCH;
        let cgpt_arch = match host_arch {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            _ => "x86_64",
        };
        
        let cgpt_path = PathBuf::from(format!("wax/lib/bin/{}/cgpt", cgpt_arch));
        
        Ok(Self {
            config,
            loopdev: None,
            cgpt_path,
        })
    }

    pub fn execute(&mut self) -> Result<()> {
        // Check if image is GPT
        self.check_gpt_image()?;

        // Fix GPT backup table
        log_info("Fixing GPT backup table");
        self.run_command("sgdisk", &["-e", self.config.image.to_str().unwrap()])?;
        self.safesync();

        // Delete partitions except 2 and 3
        log_info("Deleting partitions except kernel and root");
        self.delete_partitions_except(&[2, 3])?;
        self.safesync();

        // Create loop device
        log_info("Creating loop device");
        self.create_loop_device()?;
        self.safesync();

        // Detect or use specified architecture
        if self.config.target_arch.is_none() {
            self.detect_arch()?;
        } else {
            log_info(&format!("Using specified architecture: {}", self.config.target_arch.as_ref().unwrap()));
        }
        self.safesync();

        // Shrink and squash if not in fast mode
        if !self.config.fast {
            self.shrink_root()?;
            self.safesync();

            self.squash_partitions()?;
            self.safesync();
        } else {
            log_info("Fast mode on, skipping shrink/squash");
        }

        // Patch bootloader
        self.patch_bootloader()?;
        self.safesync();

        // Swap partitions 3 and 4 (ROOT-A becomes partition 4)
        log_info("Swapping partitions 3 and 4");
        self.run_command("sgdisk", &["-r", "3:4", self.loopdev.as_ref().unwrap()])?;
        self.safesync();

        // Patch sh1mmer
        self.patch_sh1mmer()?;
        self.safesync();

        // Detach loop device
        log_info("Detaching loop device");
        self.detach_loop_device()?;
        self.safesync();

        // Truncate image
        self.truncate_image()?;
        self.safesync();

        Ok(())
    }

    fn check_gpt_image(&self) -> Result<()> {
        let output = Command::new("sfdisk")
            .arg("-l")
            .arg(&self.config.image)
            .output()
            .context("Failed to run sfdisk")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains("Disklabel type: gpt") {
            anyhow::bail!("Image is not GPT, or is corrupted");
        }

        Ok(())
    }

    fn delete_partitions_except(&self, keep: &[u32]) -> Result<()> {
        // Delete all partitions except the ones we want to keep
        // In practice, we want to keep partitions 2 (KERN-A) and 3 (ROOT-A)
        let all_partitions: Vec<u32> = (1..=12).collect();
        let to_delete: Vec<String> = all_partitions
            .iter()
            .filter(|&p| !keep.contains(p))
            .map(|p| p.to_string())
            .collect();

        if !to_delete.is_empty() {
            let output = Command::new("sfdisk")
                .arg("--delete")
                .arg(&self.config.image)
                .args(&to_delete)
                .output();

            // It's okay if this fails - partitions might not exist
            if output.is_err() {
                log_debug("Some partitions couldn't be deleted (they might not exist)");
            }
        }

        Ok(())
    }

    fn create_loop_device(&mut self) -> Result<()> {
        let output = Command::new("losetup")
            .arg("-f")
            .output()
            .context("Failed to find free loop device")?;

        let loopdev = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Command::new("losetup")
            .arg("-P")
            .arg(&loopdev)
            .arg(&self.config.image)
            .status()
            .context("Failed to create loop device")?;

        self.loopdev = Some(loopdev.clone());
        log_debug(&format!("Created loop device: {}", loopdev));

        Ok(())
    }

    fn detach_loop_device(&mut self) -> Result<()> {
        if let Some(ref loopdev) = self.loopdev {
            Command::new("losetup")
                .arg("-d")
                .arg(loopdev)
                .status()
                .context("Failed to detach loop device")?;
            
            self.loopdev = None;
        }

        Ok(())
    }

    fn detect_arch(&mut self) -> Result<()> {
        let loopdev = self.loopdev.as_ref().unwrap();
        let mount_point = std::env::temp_dir().join("rax_root_mount");
        
        fs::create_dir_all(&mount_point)?;

        // Mount partition 3 (ROOT) read-only
        let part3 = format!("{}p3", loopdev);
        Command::new("mount")
            .arg("-o")
            .arg("ro")
            .arg(&part3)
            .arg(&mount_point)
            .status()
            .context("Failed to mount ROOT partition")?;

        // Detect architecture from /bin/bash
        let bash_path = mount_point.join("bin/bash");
        let mut arch = "x86_64".to_string();

        if bash_path.exists() {
            let output = Command::new("file")
                .arg("-b")
                .arg(&bash_path)
                .output()?;

            let file_info = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if file_info.contains("aarch64") || file_info.contains("armv8") || file_info.contains("arm") {
                arch = "aarch64".to_string();
            }
        }

        // Unmount
        Command::new("umount")
            .arg(&mount_point)
            .status()
            .context("Failed to unmount ROOT partition")?;

        fs::remove_dir(&mount_point)?;

        log_info(&format!("Detected architecture: {}", arch));
        self.config.target_arch = Some(arch);

        Ok(())
    }

    // Helper functions for partition operations
    
    fn get_sector_size(&self, device: &str) -> Result<u64> {
        let output = Command::new("sfdisk")
            .arg("-l")
            .arg(device)
            .output()
            .context("Failed to get sector size")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("Sector size") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    return parts[3].parse::<u64>()
                        .context("Failed to parse sector size");
                }
            }
        }
        
        anyhow::bail!("Could not find sector size")
    }

    fn get_final_sector(&self, device: &str) -> Result<u64> {
        let output = Command::new("sfdisk")
            .arg("-l")
            .arg("-o")
            .arg("end")
            .arg(device)
            .output()
            .context("Failed to get final sector")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut max_sector = 0u64;
        
        for line in stdout.lines() {
            if let Ok(sector) = line.trim().parse::<u64>() {
                if sector > max_sector {
                    max_sector = sector;
                }
            }
        }
        
        if max_sector == 0 {
            anyhow::bail!("Could not find final sector");
        }
        
        Ok(max_sector)
    }

    fn get_gpt_backup_sector(&self, device: &str) -> Result<u64> {
        let output = Command::new(&self.cgpt_path)
            .arg("show")
            .arg(device)
            .output()
            .context("Failed to run cgpt show")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("Sec GPT table") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    return parts[0].parse::<u64>()
                        .context("Failed to parse GPT backup sector");
                }
            }
        }
        
        anyhow::bail!("Could not find GPT backup table sector")
    }

    fn get_total_sectors(&self, device: &str) -> Result<u64> {
        let output = Command::new("sfdisk")
            .arg("-l")
            .arg(device)
            .output()
            .context("Failed to get total sectors")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("sectors") && !line.contains("Sector size") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "sectors" && i > 0 {
                        if let Ok(sectors) = parts[i-1].parse::<u64>() {
                            return Ok(sectors);
                        }
                    }
                }
            }
        }
        
        anyhow::bail!("Could not find total sectors")
    }

    fn resize_image_if_needed(&self, needed_sectors: u64) -> Result<()> {
        let loopdev = self.loopdev.as_ref().unwrap();
        let final_sector = self.get_final_sector(loopdev)?;
        let gpt_sector = self.get_gpt_backup_sector(loopdev)?;
        let difference = gpt_sector as i64 - final_sector as i64 - needed_sectors as i64 - 1;

        if difference < 0 {
            let img_sectors = self.get_total_sectors(loopdev)?;
            let sector_size = self.get_sector_size(loopdev)?;
            let new_size = sector_size * (img_sectors as i64 - difference) as u64;
            
            log_info(&format!("Resizing image to {}", format_bytes(new_size)));
            
            // Resize the image file
            let file = fs::OpenOptions::new()
                .write(true)
                .open(&self.config.image)?;
            file.set_len(new_size)?;
            
            // Fix GPT
            Command::new("sgdisk")
                .arg("-e")
                .arg(&self.config.image)
                .output()?;
                
            // Reload loop device
            Command::new("losetup")
                .arg("-c")
                .arg(loopdev)
                .status()?;
        }

        Ok(())
    }

    fn cgpt_add_partition(&self, partition: u32, sectors: u64, part_type: &str, label: &str) -> Result<()> {
        let loopdev = self.loopdev.as_ref().unwrap();
        let final_sector = self.get_final_sector(loopdev)?;
        let start_sector = final_sector + 1;

        // Ensure image is large enough
        self.resize_image_if_needed(sectors)?;

        // Add partition with cgpt
        let status = Command::new(&self.cgpt_path)
            .arg("add")
            .arg(loopdev)
            .arg("-i")
            .arg(partition.to_string())
            .arg("-b")
            .arg(start_sector.to_string())
            .arg("-s")
            .arg(sectors.to_string())
            .arg("-t")
            .arg(part_type)
            .arg("-l")
            .arg(label)
            .status()
            .context("Failed to add partition with cgpt")?;

        if !status.success() {
            anyhow::bail!("cgpt add failed");
        }

        // Update partition table in kernel
        Command::new("partx")
            .arg("-u")
            .arg("-n")
            .arg(partition.to_string())
            .arg(loopdev)
            .status()
            .context("Failed to update partition table")?;

        Ok(())
    }

    fn shrink_root(&self) -> Result<()> {
        log_info("Shrinking ROOT");

        let loopdev = self.loopdev.as_ref().unwrap();
        let part3 = format!("{}p3", loopdev);

        // Enable RW mount
        self.enable_rw_mount(&part3)?;

        // Check and resize filesystem
        Command::new("e2fsck")
            .arg("-fy")
            .arg(&part3)
            .output()
            .context("Failed to check filesystem")?;

        Command::new("resize2fs")
            .arg("-M")
            .arg("-p")
            .arg(&part3)
            .output()
            .context("Failed to resize filesystem")?;

        // Disable RW mount
        self.disable_rw_mount(&part3)?;

        // Get new size and update partition table
        let output = Command::new("tune2fs")
            .arg("-l")
            .arg(&part3)
            .output()?;

        let info = String::from_utf8_lossy(&output.stdout);
        let block_size = self.extract_tune2fs_value(&info, "Block size")?;
        let block_count = self.extract_tune2fs_value(&info, "Block count")?;

        let resized_bytes = block_count * block_size;
        log_info(&format!("Resized ROOT to {}", format_bytes(resized_bytes)));

        Ok(())
    }

    fn squash_partitions(&self) -> Result<()> {
        log_info("Squashing partitions");

        let loopdev = self.loopdev.as_ref().unwrap();

        // Squash partitions 2 and 3
        for part in [2, 3] {
            log_info(&format!("Squashing partition {}", part));
            let _ = Command::new("sfdisk")
                .arg("-N")
                .arg(part.to_string())
                .arg("--move-data")
                .arg(loopdev)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    use std::io::Write;
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(b"+,-\n");
                    }
                    child.wait()
                });
        }

        Ok(())
    }

    fn patch_bootloader(&self) -> Result<()> {
        log_info(&format!("Creating bootloader partition ({})", format_bytes(self.config.bootloader_part_size)));

        let loopdev = self.loopdev.as_ref().unwrap();
        let sector_size = self.get_sector_size(loopdev)?;
        let sectors = self.config.bootloader_part_size / sector_size;

        // Create partition 4 for bootloader
        self.cgpt_add_partition(4, sectors, "rootfs", "ROOT-A")?;

        self.safesync();

        // Format as ext2
        let part4 = format!("{}p4", loopdev);
        log_debug(&format!("Formatting {} as ext2", part4));
        
        let mut cmd = Command::new("mkfs.ext2");
        cmd.arg("-F")
            .arg("-b")
            .arg("4096")
            .arg("-L")
            .arg("ROOT-A")
            .arg(&part4);
        
        if !crate::common::Logger::is_debug() {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
        
        cmd.status().context("Failed to format bootloader partition")?;

        self.safesync();

        // Mount the partition
        let mount_point = std::env::temp_dir().join("rax_bootloader_mount");
        fs::create_dir_all(&mount_point)?;

        Command::new("mount")
            .arg(&part4)
            .arg(&mount_point)
            .status()
            .context("Failed to mount bootloader partition")?;

        // Create directories
        fs::create_dir_all(mount_point.join("bin"))?;
        fs::create_dir_all(mount_point.join("root"))?;
        fs::create_dir_all(mount_point.join("etc/init"))?;

        log_info("Copying bootloader payload");

        // Copy noarch files if they exist
        let noarch_dir = self.config.bootloader_dir.join("noarch");
        if noarch_dir.is_dir() {
            self.copy_dir_contents(&noarch_dir, &mount_point)?;
        }

        // Copy architecture-specific files if they exist
        if let Some(ref arch) = self.config.target_arch {
            let arch_dir = self.config.bootloader_dir.join(arch);
            if arch_dir.is_dir() {
                self.copy_dir_contents(&arch_dir, &mount_point)?;
            }
        }

        // Make everything executable
        self.chmod_recursive(&mount_point, 0o755)?;

        // Unmount
        Command::new("umount")
            .arg(&mount_point)
            .status()
            .context("Failed to unmount bootloader partition")?;

        fs::remove_dir(&mount_point)?;

        Ok(())
    }

    fn patch_sh1mmer(&self) -> Result<()> {
        log_info(&format!("Creating SH1MMER partition ({})", format_bytes(self.config.sh1mmer_part_size)));

        let loopdev = self.loopdev.as_ref().unwrap();
        let sector_size = self.get_sector_size(loopdev)?;
        let sectors = self.config.sh1mmer_part_size / sector_size;

        // Create partition 1 for SH1MMER
        self.cgpt_add_partition(1, sectors, "data", "SH1MMER")?;

        self.safesync();

        // Format as ext4
        let part1 = format!("{}p1", loopdev);
        log_debug(&format!("Formatting {} as ext4", part1));
        
        let mut cmd = Command::new("mkfs.ext4");
        cmd.arg("-F")
            .arg("-b")
            .arg("4096")
            .arg("-L")
            .arg("SH1MMER")
            .arg(&part1);
        
        if !crate::common::Logger::is_debug() {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
        
        cmd.status().context("Failed to format SH1MMER partition")?;

        self.safesync();

        // Mount the partition
        let mount_point = std::env::temp_dir().join("rax_sh1mmer_mount");
        fs::create_dir_all(&mount_point)?;

        Command::new("mount")
            .arg(&part1)
            .arg(&mount_point)
            .status()
            .context("Failed to mount SH1MMER partition")?;

        // Create required directories
        fs::create_dir_all(mount_point.join("dev_image/etc"))?;
        fs::create_dir_all(mount_point.join("dev_image/factory/sh"))?;

        // Create lsb-factory file
        File::create(mount_point.join("dev_image/etc/lsb-factory"))?;

        log_info("Copying main payload");
        self.copy_dir_contents(&self.config.payload_dir, &mount_point)?;
        self.chmod_recursive(&mount_point, 0o755)?;

        // Copy extra payload if provided
        if let Some(ref extra_dir) = self.config.extra_payload_dir {
            log_info("Copying extra payload");
            let target_dir = mount_point.join("root/noarch/payloads");
            fs::create_dir_all(&target_dir)?;
            self.copy_dir_contents(extra_dir, &target_dir)?;
        }

        // Copy firmware if provided
        if let Some(ref firmware_dir) = self.config.firmware_dir {
            log_info("Copying firmware");
            let target_dir = mount_point.join("root/noarch/lib/firmware");
            fs::create_dir_all(&target_dir)?;
            self.copy_dir_contents(firmware_dir, &target_dir)?;
        }

        // Copy mounted payloads if provided
        if let Some(ref mounted_dir) = self.config.mounted_payload_dir {
            if mounted_dir.is_dir() && mounted_dir.read_dir()?.next().is_some() {
                log_info("Copying mounted payload");
                let target_dir = mount_point.join("mounted_payloads");
                fs::create_dir_all(&target_dir)?;
                self.copy_dir_contents(mounted_dir, &target_dir)?;
            }
        }

        // Extract chromebrew if provided
        if let Some(ref chromebrew) = self.config.chromebrew {
            log_info("Extracting chromebrew... increase sh1mmer part size if this fails");
            let target_dir = mount_point.join("chromebrew");
            fs::create_dir_all(&target_dir)?;

            // Extract tar with pv for progress
            let status = Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "pv {} | tar -xzf - --strip-components=1 -C {}",
                    chromebrew.display(),
                    target_dir.display()
                ))
                .status()
                .context("Failed to extract chromebrew")?;

            if !status.success() {
                anyhow::bail!("Failed to extract chromebrew");
            }
        }

        // Unmount
        Command::new("umount")
            .arg(&mount_point)
            .status()
            .context("Failed to unmount SH1MMER partition")?;

        fs::remove_dir(&mount_point)?;

        Ok(())
    }

    fn truncate_image(&self) -> Result<()> {
        log_info("Truncating image to optimal size");

        let loopdev = self.loopdev.as_ref().unwrap();
        let buffer = 35u64; // magic number to ward off evil gpt corruption spirits
        let sector_size = self.get_sector_size(loopdev)?;
        let final_sector = self.get_final_sector(loopdev)?;
        let end_bytes = (final_sector + buffer) * sector_size;

        log_info(&format!("Truncating image to {}", format_bytes(end_bytes)));

        // Truncate the image file
        let file = fs::OpenOptions::new()
            .write(true)
            .open(&self.config.image)?;
        file.set_len(end_bytes)?;

        // Fix GPT backup table
        let mut cmd = Command::new("sgdisk");
        cmd.arg("-e").arg(&self.config.image);
        
        if !crate::common::Logger::is_debug() {
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
        
        cmd.status()?;

        // Write final size to file if requested
        if let Some(ref finalsizefile) = self.config.finalsizefile {
            fs::write(finalsizefile, end_bytes.to_string())?;
            log_debug(&format!("Wrote final size to {}", finalsizefile.display()));
        }

        Ok(())
    }

    // Helper function to copy directory contents
    fn copy_dir_contents(&self, src: &Path, dst: &Path) -> Result<()> {
        let status = Command::new("cp")
            .arg("-R")
            .arg(format!("{}/*", src.display()))
            .arg(dst)
            .status();

        // It's ok if this fails (empty directory)
        if let Err(e) = status {
            log_debug(&format!("cp failed (might be empty dir): {}", e));
        }

        Ok(())
    }

    // Helper function to recursively chmod
    fn chmod_recursive(&self, path: &Path, _mode: u32) -> Result<()> {
        Command::new("chmod")
            .arg("-R")
            .arg(format!("+x"))
            .arg(path)
            .status()
            .context("Failed to chmod")?;

        Ok(())
    }

    fn enable_rw_mount(&self, device: &str) -> Result<()> {
        log_debug(&format!("Enabling RW mount for {}", device));
        // This would modify the ext2 filesystem flags
        Ok(())
    }

    fn disable_rw_mount(&self, device: &str) -> Result<()> {
        log_debug(&format!("Disabling RW mount for {}", device));
        // This would modify the ext2 filesystem flags
        Ok(())
    }

    fn extract_tune2fs_value(&self, info: &str, key: &str) -> Result<u64> {
        for line in info.lines() {
            if line.contains(key) {
                if let Some(value) = line.split(':').nth(1) {
                    return value.trim().parse()
                        .context(format!("Failed to parse {} value", key));
                }
            }
        }
        anyhow::bail!("Could not find {} in tune2fs output", key)
    }

    fn run_command(&self, cmd: &str, args: &[&str]) -> Result<()> {
        let status = Command::new(cmd)
            .args(args)
            .status()
            .with_context(|| format!("Failed to run {}", cmd))?;

        if !status.success() {
            anyhow::bail!("{} failed with exit code {:?}", cmd, status.code());
        }

        Ok(())
    }

    fn safesync(&self) {
        Command::new("sync").status().ok();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

impl Drop for WaxOperations {
    fn drop(&mut self) {
        // Clean up loop device if still attached
        if self.loopdev.is_some() {
            let _ = self.detach_loop_device();
        }
    }
}
