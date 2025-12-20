use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rax")]
#[command(about = "Rust implementation of wax - shim modifying automation tool", long_about = None)]
#[command(version)]
pub struct Args {
    /// Path to factory shim image
    #[arg(short = 'i', long = "image", required = true)]
    pub image: PathBuf,

    /// Main payload ('bw' or 'legacy')
    #[arg(short = 'p', long = "payload", default_value = "bw")]
    pub payload: String,

    /// Custom main payload directory
    #[arg(long = "payload-dir")]
    pub payload_dir: Option<PathBuf>,

    /// Partition size for payload(s) (e.g., "72M")
    #[arg(short = 's', long = "sh1mmer-part-size", default_value = "72M")]
    pub sh1mmer_part_size: String,

    /// Extra payload directory
    #[arg(short = 'e', long = "extra-payload-dir")]
    pub extra_payload_dir: Option<PathBuf>,

    /// Insert firmware from directory
    #[arg(long = "firmware-dir")]
    pub firmware_dir: Option<PathBuf>,

    /// Mounted payload directory
    #[arg(short = 'm', long = "mounted-payload-dir")]
    pub mounted_payload_dir: Option<PathBuf>,

    /// Chromebrew payload (mounted)
    #[arg(long = "chromebrew")]
    pub chromebrew: Option<PathBuf>,

    /// Path to bootloader data
    #[arg(long = "bootloader-dir")]
    pub bootloader_dir: Option<PathBuf>,

    /// Bootloader rootfs partition size (e.g., "4M")
    #[arg(long = "bootloader-part-size", default_value = "4M")]
    pub bootloader_part_size: String,

    /// Force architecture for target device (x86_64 or aarch64)
    #[arg(long = "arch")]
    pub arch: Option<String>,

    /// Print debug messages
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    /// Fast/dirty build, larger image size
    #[arg(long = "fast")]
    pub fast: bool,

    /// Write final image size in bytes to this file
    #[arg(long = "finalsizefile")]
    pub finalsizefile: Option<PathBuf>,
}
