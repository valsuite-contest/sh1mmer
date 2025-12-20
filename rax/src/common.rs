use colored::Colorize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static INIT_LOGGER: Once = Once::new();
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

pub struct Logger;

impl Logger {
    pub fn init(debug: bool) {
        INIT_LOGGER.call_once(|| {
            DEBUG_MODE.store(debug, Ordering::Relaxed);
        });
    }

    pub fn is_debug() -> bool {
        DEBUG_MODE.load(Ordering::Relaxed)
    }
}

pub fn log_info(msg: &str) {
    println!("{} {}", "Info:".green(), msg);
}

pub fn log_warn(msg: &str) {
    eprintln!("{} {}", "Warning:".yellow(), msg);
}

pub fn log_error(msg: &str) {
    eprintln!("{} {}", "Error:".bright_red().bold(), msg);
}

pub fn log_debug(msg: &str) {
    if Logger::is_debug() {
        eprintln!("{} {}", "Debug:".yellow(), msg);
    }
}

/// Parse size strings like "72M", "4G", "1024" into bytes
pub fn parse_bytes(size_str: &str) -> anyhow::Result<u64> {
    let size_str = size_str.trim();
    
    // Try to parse as plain number first
    if let Ok(bytes) = size_str.parse::<u64>() {
        return Ok(bytes);
    }

    // Parse with suffix (K, M, G, etc.)
    let re = regex::Regex::new(r"^(\d+(?:\.\d+)?)\s*([KMGT]?)i?B?$").unwrap();
    
    if let Some(caps) = re.captures(&size_str.to_uppercase()) {
        let num: f64 = caps.get(1).unwrap().as_str().parse()?;
        let suffix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        
        let multiplier = match suffix {
            "K" => 1024u64,
            "M" => 1024 * 1024,
            "G" => 1024 * 1024 * 1024,
            "T" => 1024 * 1024 * 1024 * 1024,
            _ => 1,
        };
        
        Ok((num * multiplier as f64) as u64)
    } else {
        anyhow::bail!("Could not parse size '{}'", size_str)
    }
}

/// Format bytes into human-readable format
pub fn format_bytes(bytes: u64) -> String {
    byte_unit::Byte::from_u64(bytes)
        .get_appropriate_unit(byte_unit::UnitType::Binary)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bytes() {
        assert_eq!(parse_bytes("1024").unwrap(), 1024);
        assert_eq!(parse_bytes("1K").unwrap(), 1024);
        assert_eq!(parse_bytes("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_bytes("72M").unwrap(), 72 * 1024 * 1024);
        assert_eq!(parse_bytes("4M").unwrap(), 4 * 1024 * 1024);
        assert_eq!(parse_bytes("1G").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_format_bytes() {
        assert!(format_bytes(1024).contains("1"));
        assert!(format_bytes(1024 * 1024).contains("1"));
    }
}
