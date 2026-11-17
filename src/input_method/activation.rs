use log::{info, warn};
use std::process::Command;

pub fn set_ibus_engine(engine: &str) -> bool {
    let output = Command::new("ibus").args(["engine", engine]).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                info!("Switched IBus engine to {}", engine);
                true
            } else {
                warn!(
                    "Failed to switch IBus engine: {:?}",
                    String::from_utf8_lossy(&out.stderr)
                );
                false
            }
        }
        Err(e) => {
            warn!("Could not run ibus command: {}", e);
            false
        }
    }
}

pub fn get_ibus_engine() -> Option<String> {
    let output = Command::new("ibus").arg("engine").output().ok()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(name)
    } else {
        None
    }
}
