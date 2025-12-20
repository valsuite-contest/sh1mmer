use anyhow::Result;

pub struct Ext2Filesystem {
    pub block_size: u64,
    pub block_count: u64,
}

impl Ext2Filesystem {
    pub fn new(block_size: u64, block_count: u64) -> Self {
        Ext2Filesystem {
            block_size,
            block_count,
        }
    }

    pub fn total_size(&self) -> u64 {
        self.block_size * self.block_count
    }
}

pub fn create_ext2(device: &str, label: &str) -> Result<()> {
    use std::process::Command;
    
    let output = Command::new("mkfs.ext2")
        .arg("-F")
        .arg("-L")
        .arg(label)
        .arg(device)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("mkfs.ext2 failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}
