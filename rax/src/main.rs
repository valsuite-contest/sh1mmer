use anyhow::Result;
use clap::Parser;
use std::process;

mod cli;
mod common;
mod operations;

use cli::Args;
use common::Logger;
use operations::WaxOperations;

const SCRIPT_DATE: &str = "2025-12-13";

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logger
    Logger::init(args.debug);

    // Print banner
    print_banner();

    // Run the main logic
    if let Err(e) = run(args) {
        common::log_error(&format!("{:#}", e));
        process::exit(1);
    }

    common::log_info("Done. Have fun!");
}

fn print_banner() {
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│ Welcome to rax, a shim modifying automation tool (Rust impl)   │");
    println!("│ Credits: CoolElectronics, Sharp_Jack, r58playz, Rafflesia, OlyB │");
    println!("│ Script date: {}                                         │", SCRIPT_DATE);
    println!("└─────────────────────────────────────────────────────────────────┘");
}

fn run(args: Args) -> Result<()> {
    // Check if running as root
    if !nix::unistd::Uid::effective().is_root() {
        anyhow::bail!("Please run as root");
    }

    // Check required dependencies
    check_dependencies()?;

    // Create operations handler
    let mut ops = WaxOperations::new(args)?;

    // Execute the build process
    ops.execute()?;

    Ok(())
}

fn check_dependencies() -> Result<()> {
    let required_deps = vec![
        "partx", "sgdisk", "mkfs.ext4", "mkfs.ext2", "tune2fs",
        "e2fsck", "resize2fs", "file", "numfmt", "pv", "tar",
    ];

    let mut missing = Vec::new();
    for dep in required_deps {
        if which::which(dep).is_err() {
            missing.push(dep);
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "The following required commands weren't found in PATH:\n{}",
            missing.join(", ")
        );
    }

    Ok(())
}
