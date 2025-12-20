use anyhow::Result;
use std::path::Path;

pub struct Payload {
    pub message: String,
}

impl Payload {
    pub fn new(message: String) -> Self {
        Payload { message }
    }

    pub fn generate_init_script(&self) -> String {
        format!(
            r#"#!/bin/sh
clear
echo "╔══════════════════════════════════════════╗"
echo "║         Minimal Boot Payload             ║"
echo "╚══════════════════════════════════════════╝"
echo ""
echo "{}"
echo ""
echo "Sleeping indefinitely..."
sleep infinity
"#,
            self.message
        )
    }
}

pub fn create_minimal_payload(mount_point: &Path, message: &str) -> Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    
    // Create directory structure
    fs::create_dir_all(mount_point.join("dev_image/etc"))?;
    fs::create_dir_all(mount_point.join("dev_image/factory/sh"))?;
    
    // Create lsb-factory (required for boot)
    std::fs::File::create(mount_point.join("dev_image/etc/lsb-factory"))?;

    // Create init script
    let payload = Payload::new(message.to_string());
    let init_content = payload.generate_init_script();
    
    let init_path = mount_point.join("init");
    fs::write(&init_path, init_content)?;
    
    // Set executable permissions
    let metadata = fs::metadata(&init_path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&init_path, permissions)?;

    Ok(())
}
