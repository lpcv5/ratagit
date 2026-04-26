use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ObserveConfig {
    pub log_path: PathBuf,
}

impl Default for ObserveConfig {
    fn default() -> Self {
        Self {
            log_path: PathBuf::from("target/ratagit.log"),
        }
    }
}

pub fn init_observability(config: &ObserveConfig) -> std::io::Result<()> {
    if let Some(parent) = config.log_path.parent() {
        create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)?;
    writeln!(file, "ratagit observe initialized")?;
    Ok(())
}
