use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::Args;
use crate::common::{format_bytes, log_debug, log_info, log_warn, parse_bytes};

pub struct WaxOperations {
    image: PathBuf,
    bootloader_dir: PathBuf,
    payload_dir: PathBuf,
    extra_payload_dir: Option<PathBuf>,
    firmware_dir: Option<PathBuf>,
    mounted_payload_dir: Option<PathBuf>,
    chromebrew: Option<PathBuf>,
    sh1mmer_part_size: u64,
    bootloader_part_size: u64,
    target_arch: Option<String>,
    fast: bool,
    finalsizefile: Option<PathBuf>,
    loopdev: Option<String>,
}

impl WaxOperations {
    pub fn new(args: Args) -> Result<Self> {
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
            loopdev: None,
        })
    }

    pub fn execute(&mut self) -> Result<()> {
        // Check if image is GPT
        self.check_gpt_image()?;

        // Fix GPT backup table
        log_info("Fixing GPT backup table");
        self.run_command("sgdisk", &["-e", self.image.to_str().unwrap()])?;
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
        if self.target_arch.is_none() {
            self.detect_arch()?;
        } else {
            log_info(&format!("Using specified architecture: {}", self.target_arch.as_ref().unwrap()));
        }
        self.safesync();

        // Shrink and squash if not in fast mode
        if !self.fast {
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
            .arg(&self.image)
            .output()
            .context("Failed to run sfdisk")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains("Disklabel type: gpt") {
            anyhow::bail!("Image is not GPT, or is corrupted");
        }

        Ok(())
    }

    fn delete_partitions_except(&self, _keep: &[u32]) -> Result<()> {
        let output = Command::new("sfdisk")
            .arg("--delete")
            .arg(&self.image)
            .args(vec!["1", "4", "5", "6", "7", "8", "9", "10", "11", "12"])
            .output();

        // It's okay if this fails - partitions might not exist
        if output.is_err() {
            log_debug("Some partitions couldn't be deleted (they might not exist)");
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
            .arg(&self.image)
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
        self.target_arch = Some(arch);

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
        log_info(&format!("Creating bootloader partition ({})", format_bytes(self.bootloader_part_size)));

        let loopdev = self.loopdev.as_ref().unwrap();
        let _part4 = format!("{}p4", loopdev);

        // This is a simplified version - the real implementation would use cgpt
        // For now, we'll log that this operation would happen
        log_warn("Bootloader partition creation not fully implemented in this version");
        log_info(&format!("Would create partition 4 with size {} and copy bootloader files", format_bytes(self.bootloader_part_size)));

        Ok(())
    }

    fn patch_sh1mmer(&self) -> Result<()> {
        log_info(&format!("Creating SH1MMER partition ({})", format_bytes(self.sh1mmer_part_size)));

        let loopdev = self.loopdev.as_ref().unwrap();
        let _part1 = format!("{}p1", loopdev);

        // This is a simplified version
        log_warn("SH1MMER partition creation not fully implemented in this version");
        log_info(&format!("Would create partition 1 with size {} and copy payload files", format_bytes(self.sh1mmer_part_size)));

        Ok(())
    }

    fn truncate_image(&self) -> Result<()> {
        log_info("Truncating image to optimal size");

        // This is a simplified version
        log_warn("Image truncation not fully implemented in this version");

        if let Some(ref finalsizefile) = self.finalsizefile {
            log_info(&format!("Would write final size to {}", finalsizefile.display()));
        }

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
