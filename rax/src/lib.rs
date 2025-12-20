//! # rax - Rust implementation of wax
//!
//! rax is a library and command-line tool for building SH1MMER shim images.
//! It provides a safe, modular API for creating custom shims.
//!
//! ## Example: Building a custom shim
//!
//! ```no_run
//! use rax::{ShimConfig, WaxOperations};
//! use std::path::PathBuf;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = ShimConfig {
//!     image: PathBuf::from("my_shim.bin"),
//!     bootloader_dir: PathBuf::from("bootstrap"),
//!     payload_dir: PathBuf::from("my_custom_payload"),
//!     extra_payload_dir: None,
//!     firmware_dir: None,
//!     mounted_payload_dir: None,
//!     chromebrew: None,
//!     sh1mmer_part_size: 72 * 1024 * 1024,  // 72MB
//!     bootloader_part_size: 4 * 1024 * 1024, // 4MB
//!     target_arch: Some("x86_64".to_string()),
//!     fast: false,
//!     finalsizefile: None,
//! };
//!
//! let mut ops = WaxOperations::with_config(config)?;
//! ops.execute()?;
//! # Ok(())
//! # }
//! ```

pub mod cli;
pub mod common;
pub mod operations;

// Re-export main types for easy library use
pub use operations::{ShimConfig, WaxOperations};
pub use common::{log_info, log_error, log_debug, log_warn};
