use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MARKER_FILE: &str = ".ratagit-large-repo-marker";
pub const MANIFEST_FILE: &str = ".ratagit-large-repo.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scale {
    Smoke,
    Small,
    Medium,
    Large,
    Huge,
}

impl Scale {
    pub const DEFAULT_SUITE: [Self; 4] = [Self::Smoke, Self::Small, Self::Medium, Self::Large];

    pub fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "smoke" => Ok(Self::Smoke),
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            "huge" => Ok(Self::Huge),
            _ => Err(ConfigError::InvalidScale(value.to_string())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Huge => "huge",
        }
    }

    fn defaults(self) -> ScaleDefaults {
        match self {
            Self::Smoke => ScaleDefaults {
                files: 20,
                fanout: 4,
                file_bytes: 96,
                commits: 3,
                binary_files: 2,
                binary_bytes: 256,
            },
            Self::Small => ScaleDefaults {
                files: 1_000,
                fanout: 64,
                file_bytes: 96,
                commits: 25,
                binary_files: 10,
                binary_bytes: 1_024,
            },
            Self::Medium => ScaleDefaults {
                files: 10_000,
                fanout: 256,
                file_bytes: 96,
                commits: 125,
                binary_files: 100,
                binary_bytes: 4_096,
            },
            Self::Large => ScaleDefaults {
                files: 200_000,
                fanout: 1_000,
                file_bytes: 96,
                commits: 125,
                binary_files: 1_000,
                binary_bytes: 8_192,
            },
            Self::Huge => ScaleDefaults {
                files: 1_000_000,
                fanout: 2_000,
                file_bytes: 96,
                commits: 125,
                binary_files: 5_000,
                binary_bytes: 8_192,
            },
        }
    }
}

impl Display for Scale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScaleDefaults {
    files: usize,
    fanout: usize,
    file_bytes: usize,
    commits: usize,
    binary_files: usize,
    binary_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeRepoConfig {
    pub path: PathBuf,
    pub scale: Scale,
    pub files: usize,
    pub fanout: usize,
    pub file_bytes: usize,
    pub commits: usize,
    pub binary_files: usize,
    pub binary_bytes: usize,
    pub git: String,
    pub force: bool,
    pub no_commit: bool,
    pub help: bool,
}

impl LargeRepoConfig {
    pub fn for_scale(scale: Scale, path: PathBuf) -> Self {
        let defaults = scale.defaults();
        Self {
            path,
            scale,
            files: defaults.files,
            fanout: defaults.fanout,
            file_bytes: defaults.file_bytes,
            commits: defaults.commits,
            binary_files: defaults.binary_files,
            binary_bytes: defaults.binary_bytes,
            git: "git".to_string(),
            force: false,
            no_commit: false,
            help: false,
        }
    }

    pub fn parse(args: Vec<String>) -> Result<Self, ConfigError> {
        let mut scale = Scale::Large;
        let mut path = PathBuf::from("tmp/perf/large-repo");
        let mut files = None;
        let mut fanout = None;
        let mut file_bytes = None;
        let mut commits = None;
        let mut binary_files = None;
        let mut binary_bytes = None;
        let mut git = "git".to_string();
        let mut force = false;
        let mut no_commit = false;
        let mut help = false;

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--help" | "-h" => {
                    help = true;
                    index += 1;
                }
                "--scale" => {
                    scale = Scale::parse(value_after(&args, index, "--scale")?)?;
                    index += 2;
                }
                "--path" => {
                    path = PathBuf::from(value_after(&args, index, "--path")?);
                    index += 2;
                }
                "--files" => {
                    files = Some(parse_usize(
                        value_after(&args, index, "--files")?,
                        "--files",
                    )?);
                    index += 2;
                }
                "--fanout" => {
                    fanout = Some(parse_usize(
                        value_after(&args, index, "--fanout")?,
                        "--fanout",
                    )?);
                    index += 2;
                }
                "--file-bytes" => {
                    file_bytes = Some(parse_usize(
                        value_after(&args, index, "--file-bytes")?,
                        "--file-bytes",
                    )?);
                    index += 2;
                }
                "--commits" => {
                    commits = Some(parse_usize(
                        value_after(&args, index, "--commits")?,
                        "--commits",
                    )?);
                    index += 2;
                }
                "--binary-files" => {
                    binary_files = Some(parse_usize(
                        value_after(&args, index, "--binary-files")?,
                        "--binary-files",
                    )?);
                    index += 2;
                }
                "--binary-bytes" => {
                    binary_bytes = Some(parse_usize(
                        value_after(&args, index, "--binary-bytes")?,
                        "--binary-bytes",
                    )?);
                    index += 2;
                }
                "--git" => {
                    git = value_after(&args, index, "--git")?.to_string();
                    index += 2;
                }
                "--force" => {
                    force = true;
                    index += 1;
                }
                "--no-commit" => {
                    no_commit = true;
                    index += 1;
                }
                other => return Err(ConfigError::UnknownArgument(other.to_string())),
            }
        }

        let mut config = Self::for_scale(scale, path);
        config.files = files.unwrap_or(config.files);
        config.fanout = fanout.unwrap_or(config.fanout);
        config.file_bytes = file_bytes.unwrap_or(config.file_bytes);
        config.commits = commits.unwrap_or(config.commits);
        config.binary_files = binary_files.unwrap_or(config.binary_files);
        config.binary_bytes = binary_bytes.unwrap_or(config.binary_bytes);
        config.git = git;
        config.force = force;
        config.no_commit = no_commit;
        config.help = help;
        config.validate()?;
        Ok(config)
    }

    pub fn text_files(&self) -> usize {
        self.files.saturating_sub(self.binary_files)
    }

    pub fn index_entry_count(&self) -> usize {
        if self.no_commit { 0 } else { self.files + 2 }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.help {
            return Ok(());
        }
        if self.files == 0 {
            return Err(ConfigError::InvalidValue("--files must be greater than 0"));
        }
        if self.fanout == 0 {
            return Err(ConfigError::InvalidValue("--fanout must be greater than 0"));
        }
        if self.file_bytes == 0 {
            return Err(ConfigError::InvalidValue(
                "--file-bytes must be greater than 0",
            ));
        }
        if self.commits == 0 {
            return Err(ConfigError::InvalidValue(
                "--commits must be greater than 0",
            ));
        }
        if self.commits > self.files {
            return Err(ConfigError::InvalidValue(
                "--commits must be less than or equal to --files",
            ));
        }
        if self.binary_files > self.files {
            return Err(ConfigError::InvalidValue(
                "--binary-files must be less than or equal to --files",
            ));
        }
        if self.binary_files > 0 && self.binary_bytes == 0 {
            return Err(ConfigError::InvalidValue(
                "--binary-bytes must be greater than 0 when binary files are requested",
            ));
        }
        if self.git.trim().is_empty() {
            return Err(ConfigError::InvalidValue("--git must not be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeRepoManifest {
    pub scale: Scale,
    pub files: usize,
    pub text_files: usize,
    pub binary_files: usize,
    pub fanout: usize,
    pub file_bytes: usize,
    pub binary_bytes: usize,
    pub commits: usize,
    pub committed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingValue(&'static str),
    InvalidNumber { option: &'static str, value: String },
    InvalidValue(&'static str),
    InvalidScale(String),
    UnknownArgument(String),
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue(option) => write!(formatter, "missing value for {option}"),
            Self::InvalidNumber { option, value } => {
                write!(formatter, "invalid number for {option}: {value}")
            }
            Self::InvalidValue(message) => formatter.write_str(message),
            Self::InvalidScale(scale) => write!(formatter, "invalid scale: {scale}"),
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument: {argument}"),
        }
    }
}

impl Error for ConfigError {}

pub fn make_large_repo_usage() -> &'static str {
    "Usage: cargo run --bin make-large-repo -- [options]\n\
\n\
Options:\n\
  --scale <name>         smoke, small, medium, large, or huge (default: large)\n\
  --path <path>          Target repository path (default: tmp/perf/large-repo)\n\
  --files <count>        Total tracked files to generate\n\
  --fanout <count>       Number of bucket directories per commit group\n\
  --file-bytes <count>   Bytes per text file\n\
  --commits <count>      Total commits in the generated repository\n\
  --binary-files <count> Number of tracked binary files within --files\n\
  --binary-bytes <count> Bytes per binary file\n\
  --git <path>           Git executable (default: git)\n\
  --force                Regenerate a repo created by this tool\n\
  --no-commit            Generate files but leave them untracked\n\
  -h, --help             Print this help text"
}

pub fn generate_repo(config: &LargeRepoConfig) -> Result<LargeRepoManifest, Box<dyn Error>> {
    prepare_target(config)?;
    initialize_repo(config)?;
    write_marker_and_manifest(config, !config.no_commit)?;
    generate_files(config)?;
    stage_and_commit(config)?;
    Ok(manifest_for_config(config, !config.no_commit))
}

pub fn manifest_for_config(config: &LargeRepoConfig, committed: bool) -> LargeRepoManifest {
    LargeRepoManifest {
        scale: config.scale,
        files: config.files,
        text_files: config.text_files(),
        binary_files: config.binary_files,
        fanout: config.fanout,
        file_bytes: config.file_bytes,
        binary_bytes: config.binary_bytes,
        commits: config.commits,
        committed,
    }
}

pub fn text_repo_path(config: &LargeRepoConfig, text_index: usize) -> Option<String> {
    if text_index >= config.text_files() {
        return None;
    }
    Some(repo_path_for_file(
        FileKind::Text,
        text_index,
        config.binary_files + text_index,
        config,
    ))
}

pub fn binary_repo_path(config: &LargeRepoConfig, binary_index: usize) -> Option<String> {
    if binary_index >= config.binary_files {
        return None;
    }
    Some(repo_path_for_file(
        FileKind::Binary,
        binary_index,
        binary_index,
        config,
    ))
}

fn value_after<'a>(
    args: &'a [String],
    index: usize,
    option: &'static str,
) -> Result<&'a str, ConfigError> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or(ConfigError::MissingValue(option))
}

fn parse_usize(value: &str, option: &'static str) -> Result<usize, ConfigError> {
    value.parse().map_err(|_| ConfigError::InvalidNumber {
        option,
        value: value.to_string(),
    })
}

fn prepare_target(config: &LargeRepoConfig) -> Result<(), Box<dyn Error>> {
    if config.path.exists() {
        if !config.force {
            return Err(format!(
                "{} already exists; pass --force to regenerate a previously created synthetic repo",
                config.path.display()
            )
            .into());
        }
        let marker = config.path.join(MARKER_FILE);
        if !marker.is_file() {
            return Err(format!(
                "refusing to remove {} because {} is missing",
                config.path.display(),
                MARKER_FILE
            )
            .into());
        }
        fs::remove_dir_all(&config.path)?;
    }

    fs::create_dir_all(&config.path)?;
    Ok(())
}

fn initialize_repo(config: &LargeRepoConfig) -> Result<(), Box<dyn Error>> {
    run_git(config, ["init", "-q", "-b", "main"])?;
    run_git(config, ["config", "core.autocrlf", "false"])?;
    run_git(config, ["config", "gc.auto", "0"])?;
    Ok(())
}

fn write_marker_and_manifest(
    config: &LargeRepoConfig,
    committed: bool,
) -> Result<(), Box<dyn Error>> {
    fs::write(
        config.path.join(MARKER_FILE),
        "ratagit synthetic large repo\n",
    )?;
    fs::write(
        config.path.join(MANIFEST_FILE),
        manifest_json(&manifest_for_config(config, committed)),
    )?;
    Ok(())
}

fn generate_files(config: &LargeRepoConfig) -> Result<(), Box<dyn Error>> {
    for index in 0..config.binary_files {
        let Some(repo_path) = binary_repo_path(config, index) else {
            continue;
        };
        let path = config.path.join(repo_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_deterministic_binary_file(&path, index, config.binary_bytes)?;
        report_generation_progress(index + 1, config.files);
    }

    for index in 0..config.text_files() {
        let Some(repo_path) = text_repo_path(config, index) else {
            continue;
        };
        let path = config.path.join(repo_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_deterministic_text_file(&path, index, config.file_bytes)?;
        report_generation_progress(config.binary_files + index + 1, config.files);
    }

    println!("generated {} files", config.files);
    Ok(())
}

fn report_generation_progress(done: usize, total: usize) {
    if done < total && done.is_multiple_of(10_000) {
        println!("generated {done} files");
    }
}

#[derive(Debug, Clone, Copy)]
enum FileKind {
    Text,
    Binary,
}

fn repo_path_for_file(
    kind: FileKind,
    kind_index: usize,
    global_index: usize,
    config: &LargeRepoConfig,
) -> String {
    let group = global_index % config.commits;
    let bucket = kind_index % config.fanout;
    match kind {
        FileKind::Text => {
            format!("data/text/c{group:04}/{bucket:04}/file-{kind_index:09}.txt")
        }
        FileKind::Binary => {
            format!("data/binary/c{group:04}/{bucket:04}/blob-{kind_index:09}.bin")
        }
    }
}

fn write_deterministic_text_file(path: &Path, index: usize, file_bytes: usize) -> io::Result<()> {
    let seed = format!("ratagit synthetic text file {index:09}\n");
    let mut file = File::create(path)?;
    let mut remaining = file_bytes;
    while remaining > 0 {
        let bytes = seed.as_bytes();
        let chunk_len = remaining.min(bytes.len());
        file.write_all(&bytes[..chunk_len])?;
        remaining -= chunk_len;
    }
    Ok(())
}

fn write_deterministic_binary_file(
    path: &Path,
    index: usize,
    binary_bytes: usize,
) -> io::Result<()> {
    let mut file = File::create(path)?;
    let mut written = 0usize;
    let mut buffer = [0u8; 8192];
    while written < binary_bytes {
        let chunk_len = (binary_bytes - written).min(buffer.len());
        for (offset, byte) in buffer[..chunk_len].iter_mut().enumerate() {
            let position = written + offset;
            *byte = if position.is_multiple_of(97) {
                0
            } else {
                ((index + position * 31) % 251 + 1) as u8
            };
        }
        file.write_all(&buffer[..chunk_len])?;
        written += chunk_len;
    }
    Ok(())
}

fn stage_and_commit(config: &LargeRepoConfig) -> Result<(), Box<dyn Error>> {
    if config.no_commit {
        println!("leaving files untracked because --no-commit was passed");
        return Ok(());
    }

    for group in 0..config.commits {
        if group == 0 {
            run_git(config, ["add", MARKER_FILE, MANIFEST_FILE])?;
        }
        let text_group = format!("data/text/c{group:04}");
        if config.path.join(&text_group).is_dir() {
            run_git(config, ["add", text_group.as_str()])?;
        }
        let binary_group = format!("data/binary/c{group:04}");
        if config.path.join(&binary_group).is_dir() {
            run_git(config, ["add", binary_group.as_str()])?;
        }
        let message = format!("chore(repo): add synthetic group {group:04}");
        run_git(config, ["commit", "--quiet", "-m", message.as_str()])?;
        println!("committed synthetic group {group} of {}", config.commits);
    }
    Ok(())
}

pub fn run_git<I, S>(config: &LargeRepoConfig, args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new(&config.git)
        .current_dir(&config.path)
        .args([
            "-c",
            "user.name=ratagit",
            "-c",
            "user.email=ratagit@example.invalid",
        ])
        .args(args)
        .status()?;

    if !status.success() {
        return Err(format!("git command failed with status {status}").into());
    }
    Ok(())
}

pub fn manifest_json(manifest: &LargeRepoManifest) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema\": \"ratagit.synthetic-repo.v1\",\n",
            "  \"scale\": \"{}\",\n",
            "  \"files\": {},\n",
            "  \"text_files\": {},\n",
            "  \"binary_files\": {},\n",
            "  \"fanout\": {},\n",
            "  \"file_bytes\": {},\n",
            "  \"binary_bytes\": {},\n",
            "  \"commits\": {},\n",
            "  \"committed\": {}\n",
            "}}\n"
        ),
        manifest.scale.as_str(),
        manifest.files,
        manifest.text_files,
        manifest.binary_files,
        manifest.fanout,
        manifest.file_bytes,
        manifest.binary_bytes,
        manifest.commits,
        manifest.committed
    )
}

pub fn print_large_repo_summary(config: &LargeRepoConfig, manifest: &LargeRepoManifest) {
    println!("synthetic repository ready: {}", config.path.display());
    println!("scale: {}", manifest.scale);
    println!("files: {}", manifest.files);
    println!("text files: {}", manifest.text_files);
    println!("binary files: {}", manifest.binary_files);
    println!("fanout directories: {}", manifest.fanout);
    println!("text bytes per file: {}", manifest.file_bytes);
    println!("binary bytes per file: {}", manifest.binary_bytes);
    println!("commits: {}", manifest.commits);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let config = LargeRepoConfig::parse(Vec::new()).expect("default config should parse");

        assert_eq!(config.path, PathBuf::from("tmp/perf/large-repo"));
        assert_eq!(config.scale, Scale::Large);
        assert_eq!(config.files, 200_000);
        assert_eq!(config.fanout, 1_000);
        assert_eq!(config.file_bytes, 96);
        assert_eq!(config.commits, 125);
        assert_eq!(config.binary_files, 1_000);
        assert_eq!(config.binary_bytes, 8_192);
        assert_eq!(config.git, "git");
        assert!(!config.force);
        assert!(!config.no_commit);
    }

    #[test]
    fn scale_defaults_can_be_overridden() {
        let config = LargeRepoConfig::parse(vec![
            "--scale".into(),
            "smoke".into(),
            "--path".into(),
            "tmp/perf/custom".into(),
            "--files".into(),
            "12".into(),
            "--fanout".into(),
            "3".into(),
            "--file-bytes".into(),
            "17".into(),
            "--commits".into(),
            "4".into(),
            "--binary-files".into(),
            "5".into(),
            "--binary-bytes".into(),
            "19".into(),
            "--git".into(),
            "custom-git".into(),
            "--force".into(),
            "--no-commit".into(),
        ])
        .expect("explicit config should parse");

        assert_eq!(config.scale, Scale::Smoke);
        assert_eq!(config.path, PathBuf::from("tmp/perf/custom"));
        assert_eq!(config.files, 12);
        assert_eq!(config.text_files(), 7);
        assert_eq!(config.fanout, 3);
        assert_eq!(config.file_bytes, 17);
        assert_eq!(config.commits, 4);
        assert_eq!(config.binary_files, 5);
        assert_eq!(config.binary_bytes, 19);
        assert_eq!(config.git, "custom-git");
        assert!(config.force);
        assert!(config.no_commit);
    }

    #[test]
    fn rejects_binary_file_count_above_total_files() {
        let error = LargeRepoConfig::parse(vec![
            "--files".into(),
            "2".into(),
            "--commits".into(),
            "1".into(),
            "--binary-files".into(),
            "3".into(),
        ])
        .expect_err("too many binary files should be rejected");

        assert_eq!(
            error,
            ConfigError::InvalidValue("--binary-files must be less than or equal to --files")
        );
    }

    #[test]
    fn rejects_more_commits_than_files() {
        let error = LargeRepoConfig::parse(vec![
            "--files".into(),
            "2".into(),
            "--commits".into(),
            "3".into(),
        ])
        .expect_err("too many commits should be rejected");

        assert_eq!(
            error,
            ConfigError::InvalidValue("--commits must be less than or equal to --files")
        );
    }

    #[test]
    fn maps_text_and_binary_paths_deterministically() {
        let config = LargeRepoConfig::parse(vec![
            "--path".into(),
            "repo".into(),
            "--files".into(),
            "20".into(),
            "--binary-files".into(),
            "5".into(),
            "--fanout".into(),
            "4".into(),
            "--commits".into(),
            "3".into(),
        ])
        .expect("config should parse");

        assert_eq!(
            binary_repo_path(&config, 4).as_deref(),
            Some("data/binary/c0001/0000/blob-000000004.bin")
        );
        assert_eq!(
            text_repo_path(&config, 7).as_deref(),
            Some("data/text/c0000/0003/file-000000007.txt")
        );
    }

    #[test]
    fn manifest_json_is_stable() {
        let config = LargeRepoConfig::parse(vec![
            "--scale".into(),
            "smoke".into(),
            "--files".into(),
            "10".into(),
            "--commits".into(),
            "2".into(),
            "--binary-files".into(),
            "1".into(),
        ])
        .expect("config should parse");

        let json = manifest_json(&manifest_for_config(&config, true));

        assert!(json.contains("\"schema\": \"ratagit.synthetic-repo.v1\""));
        assert!(json.contains("\"scale\": \"smoke\""));
        assert!(json.contains("\"files\": 10"));
        assert!(json.contains("\"commits\": 2"));
        assert!(json.contains("\"committed\": true"));
    }
}
