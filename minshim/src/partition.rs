use anyhow::Result;
use std::process::Command;

pub struct PartitionTable {
    pub partitions: Vec<Partition>,
}

pub struct Partition {
    pub number: u32,
    pub name: String,
}

impl PartitionTable {
    pub fn read(device: &str) -> Result<Self> {
        let output = Command::new("sfdisk")
            .arg("-l")
            .arg(device)
            .output()?;

        let info = String::from_utf8_lossy(&output.stdout);
        
        let mut partitions = Vec::new();
        for line in info.lines() {
            if line.contains(device) && line.contains("Linux") {
                // Parse partition info
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Some(num_str) = parts[0].strip_prefix(device).and_then(|s| s.strip_prefix("p")) {
                        if let Ok(num) = num_str.parse::<u32>() {
                            partitions.push(Partition {
                                number: num,
                                name: format!("partition {}", num),
                            });
                        }
                    }
                }
            }
        }

        Ok(PartitionTable { partitions })
    }

    pub fn print_summary(&self) {
        println!("      Found {} partitions", self.partitions.len());
        for part in &self.partitions {
            println!("      - Partition {}: {}", part.number, part.name);
        }
    }
}
