use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::util::SubscriberInitExt;

const DEFAULT_FILTER: &str = "info";
const TRACE_FILTER: &str = "trace";

#[derive(Debug)]
pub struct ObservabilityGuard {
    _guard: tracing_appender::non_blocking::WorkerGuard,
}

#[derive(Debug, Clone)]
pub struct ObserveConfig {
    pub log_path: PathBuf,
    pub filter: String,
}

impl Default for ObserveConfig {
    fn default() -> Self {
        Self {
            log_path: default_log_path(),
            filter: DEFAULT_FILTER.to_string(),
        }
    }
}

impl ObserveConfig {
    pub fn from_env() -> Self {
        Self::from_env_vars(|name| env::var(name).ok())
    }

    fn from_env_vars(mut get_env: impl FnMut(&str) -> Option<String>) -> Self {
        let log_path = get_env("RATAGIT_LOG_PATH")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_log_path);
        let filter = get_env("RATAGIT_LOG")
            .filter(|value| !value.trim().is_empty())
            .or_else(|| trace_env_enabled(&mut get_env).then(|| TRACE_FILTER.to_string()))
            .unwrap_or_else(|| DEFAULT_FILTER.to_string());

        Self { log_path, filter }
    }
}

pub fn init_observability(config: &ObserveConfig) -> std::io::Result<ObservabilityGuard> {
    if let Some(parent) = config.log_path.parent() {
        create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    let filter = env_filter(&config.filter);
    let subscriber = SubscriberBuilder::default()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_span_events(FmtSpan::NONE)
        .finish();

    let _ = subscriber.try_init();
    tracing::info!(
        target: "ratagit.observe",
        path = %config.log_path.display(),
        filter = %config.filter,
        "ratagit observability initialized"
    );

    Ok(ObservabilityGuard { _guard: guard })
}

fn env_filter(filter: &str) -> EnvFilter {
    EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER))
}

fn trace_env_enabled(get_env: &mut impl FnMut(&str) -> Option<String>) -> bool {
    get_env("RATAGIT_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn default_log_path() -> PathBuf {
    default_state_dir().join("ratagit").join("ratagit.log")
}

#[cfg(windows)]
fn default_state_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| {
            env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData").join("Local"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(not(windows))]
fn default_state_dir() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("state"))
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config_from(pairs: &[(&str, &str)]) -> ObserveConfig {
        let vars = pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();
        ObserveConfig::from_env_vars(|name| vars.get(name).cloned())
    }

    #[test]
    fn default_config_uses_info_and_state_log_path() {
        let config = config_from(&[]);

        assert_eq!(config.filter, "info");
        assert!(config.log_path.ends_with("ratagit/ratagit.log"));
        assert!(!config.log_path.starts_with("target"));
    }

    #[test]
    fn env_overrides_filter_and_log_path() {
        let config = config_from(&[
            ("RATAGIT_LOG", "ratagit=debug,ratagit_git=trace"),
            ("RATAGIT_LOG_PATH", "custom/ratagit.log"),
        ]);

        assert_eq!(config.filter, "ratagit=debug,ratagit_git=trace");
        assert_eq!(config.log_path, PathBuf::from("custom/ratagit.log"));
    }

    #[test]
    fn trace_env_enables_trace_when_log_is_unset() {
        let config = config_from(&[("RATAGIT_TRACE", "1")]);

        assert_eq!(config.filter, "trace");
    }

    #[test]
    fn explicit_log_wins_over_trace_env() {
        let config = config_from(&[("RATAGIT_TRACE", "1"), ("RATAGIT_LOG", "debug")]);

        assert_eq!(config.filter, "debug");
    }
}
