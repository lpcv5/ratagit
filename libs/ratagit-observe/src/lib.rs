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

#[cfg(test)]
mod tests {
    use std::fs::{read_to_string, remove_dir_all};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn default_log_path_stays_under_target() {
        assert_eq!(
            ObserveConfig::default().log_path,
            PathBuf::from("target/ratagit.log")
        );
    }

    #[test]
    fn init_observability_creates_parent_and_appends_marker_line() {
        let root = std::env::temp_dir().join(format!(
            "ratagit-observe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
        let log_path = root.join("nested").join("ratagit.log");
        let config = ObserveConfig {
            log_path: log_path.clone(),
        };

        init_observability(&config).expect("first init should create log");
        init_observability(&config).expect("second init should append log");

        let content = read_to_string(&log_path).expect("log should be readable");
        assert_eq!(
            content.lines().collect::<Vec<_>>(),
            vec!["ratagit observe initialized", "ratagit observe initialized"]
        );
        let _ = remove_dir_all(root);
    }
}
