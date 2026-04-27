#[path = "../perf_repo.rs"]
#[allow(dead_code)]
mod perf_repo;

use std::env;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use perf_repo::{
    LargeRepoConfig, LargeRepoManifest, MANIFEST_FILE, MARKER_FILE, Scale, generate_repo,
    manifest_for_config, text_repo_path,
};
use ratagit_core::{
    COMMITS_PAGE_SIZE, CommitFileDiffPath, CommitFileDiffTarget, CommitFileEntry, CommitFileStatus,
    FileDiffTarget, FileEntry,
};
use ratagit_git::{GitBackend, HybridGitBackend};

const DEFAULT_ITERATIONS: usize = 3;
const DEFAULT_WARMUP: usize = 1;

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let config = PerfConfig::parse(args)?;
    if config.help {
        println!("{}", perf_suite_usage());
        return Ok(());
    }

    let report = run_suite(&config)?;
    println!("performance suite completed: {}", report.run_id);
    println!("json: {}", report.json_path.display());
    println!("markdown: {}", report.markdown_path.display());
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PerfConfig {
    root: PathBuf,
    output: PathBuf,
    scales: Vec<Scale>,
    operations: Vec<Operation>,
    iterations: usize,
    warmup: usize,
    git: String,
    regenerate: bool,
    help: bool,
}

impl PerfConfig {
    fn parse(args: Vec<String>) -> Result<Self, PerfConfigError> {
        let mut config = Self {
            root: PathBuf::from("tmp/perf/suite"),
            output: PathBuf::from("tmp/perf/results"),
            scales: Scale::DEFAULT_SUITE.to_vec(),
            operations: Operation::ALL.to_vec(),
            iterations: DEFAULT_ITERATIONS,
            warmup: DEFAULT_WARMUP,
            git: "git".to_string(),
            regenerate: false,
            help: false,
        };

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--help" | "-h" => {
                    config.help = true;
                    index += 1;
                }
                "--root" => {
                    config.root = PathBuf::from(value_after(&args, index, "--root")?);
                    index += 2;
                }
                "--output" => {
                    config.output = PathBuf::from(value_after(&args, index, "--output")?);
                    index += 2;
                }
                "--scales" => {
                    config.scales = parse_scales(value_after(&args, index, "--scales")?)?;
                    index += 2;
                }
                "--operations" => {
                    config.operations =
                        parse_operations(value_after(&args, index, "--operations")?)?;
                    index += 2;
                }
                "--iterations" => {
                    config.iterations =
                        parse_usize(value_after(&args, index, "--iterations")?, "--iterations")?;
                    index += 2;
                }
                "--warmup" => {
                    config.warmup =
                        parse_usize(value_after(&args, index, "--warmup")?, "--warmup")?;
                    index += 2;
                }
                "--git" => {
                    config.git = value_after(&args, index, "--git")?.to_string();
                    index += 2;
                }
                "--regenerate" => {
                    config.regenerate = true;
                    index += 1;
                }
                other => return Err(PerfConfigError::UnknownArgument(other.to_string())),
            }
        }

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), PerfConfigError> {
        if self.help {
            return Ok(());
        }
        if self.scales.is_empty() {
            return Err(PerfConfigError::InvalidValue(
                "--scales must include at least one scale",
            ));
        }
        if self.operations.is_empty() {
            return Err(PerfConfigError::InvalidValue(
                "--operations must include at least one operation",
            ));
        }
        if self.iterations == 0 {
            return Err(PerfConfigError::InvalidValue(
                "--iterations must be greater than 0",
            ));
        }
        if self.git.trim().is_empty() {
            return Err(PerfConfigError::InvalidValue("--git must not be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PerfConfigError {
    MissingValue(&'static str),
    InvalidNumber { option: &'static str, value: String },
    InvalidValue(&'static str),
    UnknownArgument(String),
}

impl Display for PerfConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue(option) => write!(formatter, "missing value for {option}"),
            Self::InvalidNumber { option, value } => {
                write!(formatter, "invalid number for {option}: {value}")
            }
            Self::InvalidValue(message) => formatter.write_str(message),
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument: {argument}"),
        }
    }
}

impl Error for PerfConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Operation {
    Status,
    Commits,
    LoadMoreCommits,
    CommitFiles,
    CommitDetailsDiff,
    CommitFileDiff,
    FilesDetailsDiff,
}

impl Operation {
    const ALL: [Self; 7] = [
        Self::Status,
        Self::Commits,
        Self::LoadMoreCommits,
        Self::CommitFiles,
        Self::CommitDetailsDiff,
        Self::CommitFileDiff,
        Self::FilesDetailsDiff,
    ];

    fn parse(value: &str) -> Result<Self, PerfConfigError> {
        match value {
            "status" => Ok(Self::Status),
            "commits" => Ok(Self::Commits),
            "load-more-commits" => Ok(Self::LoadMoreCommits),
            "commit-files" => Ok(Self::CommitFiles),
            "commit-details-diff" => Ok(Self::CommitDetailsDiff),
            "commit-file-diff" => Ok(Self::CommitFileDiff),
            "files-details-diff" => Ok(Self::FilesDetailsDiff),
            _ => Err(PerfConfigError::InvalidValue("unknown operation")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Commits => "commits",
            Self::LoadMoreCommits => "load-more-commits",
            Self::CommitFiles => "commit-files",
            Self::CommitDetailsDiff => "commit-details-diff",
            Self::CommitFileDiff => "commit-file-diff",
            Self::FilesDetailsDiff => "files-details-diff",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Runner {
    GitCliRaw,
    GitCliParsed,
    Backend,
}

impl Runner {
    const ALL: [Self; 3] = [Self::GitCliRaw, Self::GitCliParsed, Self::Backend];

    fn as_str(self) -> &'static str {
        match self {
            Self::GitCliRaw => "git_cli_raw",
            Self::GitCliParsed => "git_cli_parsed",
            Self::Backend => "backend",
        }
    }
}

#[derive(Debug, Clone)]
struct SuiteReport {
    run_id: String,
    git_version: String,
    repos: Vec<RepoReport>,
    records: Vec<PerfRecord>,
    json_path: PathBuf,
    markdown_path: PathBuf,
}

#[derive(Debug, Clone)]
struct RepoReport {
    scale: Scale,
    path: PathBuf,
    reused: bool,
    manifest: LargeRepoManifest,
}

#[derive(Debug, Clone)]
struct PerfRecord {
    scale: Scale,
    operation: Operation,
    runner: Runner,
    iteration: usize,
    elapsed_ms: u128,
    output_items: usize,
    output_bytes: usize,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct BenchTargets {
    head_commit: String,
    commit_file_target: CommitFileDiffTarget,
    files_diff_targets: Vec<FileDiffTarget>,
    files_diff_paths: Vec<String>,
}

struct MeasureContext<'a> {
    git: &'a str,
    config: &'a LargeRepoConfig,
    targets: &'a BenchTargets,
    backend: &'a mut HybridGitBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusModeForPerf {
    Full,
    LargeRepoFast,
    HugeRepoMetadataOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MeasurementPayload {
    output_items: usize,
    output_bytes: usize,
}

fn run_suite(config: &PerfConfig) -> Result<SuiteReport, Box<dyn Error>> {
    fs::create_dir_all(&config.output)?;
    let run_id = run_id();
    let git_version = git_version(&config.git)?;
    let mut repos = Vec::new();
    let mut records = Vec::new();

    for scale in &config.scales {
        let mut repo_config = LargeRepoConfig::for_scale(*scale, config.root.join(scale.as_str()));
        repo_config.git = config.git.clone();
        let (manifest, reused) = ensure_repo(&repo_config, config.regenerate)?;
        repos.push(RepoReport {
            scale: *scale,
            path: repo_config.path.clone(),
            reused,
            manifest: manifest.clone(),
        });

        apply_dirty_layer(&repo_config)?;
        let targets = build_targets(&repo_config)?;
        let mut backend = HybridGitBackend::open(&repo_config.path)?;
        let mut context = MeasureContext {
            git: &config.git,
            config: &repo_config,
            targets: &targets,
            backend: &mut backend,
        };

        for _ in 0..config.warmup {
            for operation in &config.operations {
                for runner in Runner::ALL {
                    let _ = measure_runner(&mut context, *operation, runner);
                }
            }
        }

        for iteration in 0..config.iterations {
            for operation in &config.operations {
                for runner in Runner::ALL {
                    let record =
                        measure_record(*scale, *operation, runner, iteration, &mut context);
                    records.push(record);
                }
            }
        }
    }

    let json_path = config.output.join(format!("{run_id}.json"));
    let markdown_path = config.output.join(format!("{run_id}.md"));
    let report = SuiteReport {
        run_id,
        git_version,
        repos,
        records,
        json_path,
        markdown_path,
    };
    write_json_report(&report)?;
    write_markdown_report(&report)?;
    Ok(report)
}

fn ensure_repo(
    repo_config: &LargeRepoConfig,
    regenerate: bool,
) -> Result<(LargeRepoManifest, bool), Box<dyn Error>> {
    if repo_config.path.exists() {
        if regenerate {
            let mut config = repo_config.clone();
            config.force = true;
            return generate_repo(&config).map(|manifest| (manifest, false));
        }
        if !repo_config.path.join(MARKER_FILE).is_file()
            || !repo_config.path.join(MANIFEST_FILE).is_file()
        {
            return Err(format!(
                "{} exists but is not a synthetic repo; pass --regenerate only for marked repos",
                repo_config.path.display()
            )
            .into());
        }
        return Ok((manifest_for_config(repo_config, true), true));
    }

    generate_repo(repo_config).map(|manifest| (manifest, false))
}

fn apply_dirty_layer(config: &LargeRepoConfig) -> Result<(), Box<dyn Error>> {
    run_git_status(&config.git, &config.path, ["reset", "--hard", "HEAD"])?;
    run_git_status(&config.git, &config.path, ["clean", "-fd"])?;

    let staged = text_repo_path(config, 0)
        .or_else(|| perf_repo::binary_repo_path(config, 0))
        .ok_or("synthetic repo has no file to stage")?;
    overwrite_dirty_file(&config.path.join(&staged), "staged", 0)?;
    run_git_status(&config.git, &config.path, ["add", "--", staged.as_str()])?;

    if let Some(path) = text_repo_path(config, 1) {
        overwrite_dirty_file(&config.path.join(path), "unstaged", 1)?;
    }
    if let Some(path) = perf_repo::binary_repo_path(config, 0) {
        overwrite_dirty_binary(&config.path.join(path), 0)?;
    }

    let untracked_dir = config.path.join("untracked");
    fs::create_dir_all(&untracked_dir)?;
    fs::write(
        untracked_dir.join("perf-note.txt"),
        "ratagit perf untracked\n",
    )?;
    Ok(())
}

fn overwrite_dirty_file(path: &Path, label: &str, index: usize) -> Result<(), Box<dyn Error>> {
    fs::write(
        path,
        format!("ratagit perf {label} change {index}\nsecond line\n"),
    )?;
    Ok(())
}

fn overwrite_dirty_binary(path: &Path, index: usize) -> Result<(), Box<dyn Error>> {
    let mut bytes = vec![0u8; 256];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = if offset % 17 == 0 {
            0
        } else {
            ((index + offset * 13) % 251 + 1) as u8
        };
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn build_targets(config: &LargeRepoConfig) -> Result<BenchTargets, Box<dyn Error>> {
    let head_commit = run_git_text(&config.git, &config.path, ["rev-parse", "HEAD"])?
        .trim()
        .to_string();
    let commit_files_output = run_git_text(
        &config.git,
        &config.path,
        [
            "diff-tree",
            "--root",
            "--no-commit-id",
            "--name-status",
            "-r",
            "-M",
            "-C",
            "HEAD",
        ],
    )?;
    let commit_files = parse_commit_files(&commit_files_output)?;
    let commit_file_path = select_commit_file_path(&commit_files)?;
    let commit_file_target = CommitFileDiffTarget {
        commit_id: head_commit.clone(),
        paths: vec![CommitFileDiffPath {
            path: commit_file_path.path,
            old_path: commit_file_path.old_path,
        }],
    };

    let files_diff_paths = dirty_tracked_paths(config);
    let files_diff_targets = files_diff_paths
        .iter()
        .map(|path| FileDiffTarget {
            path: path.clone(),
            untracked: false,
            is_directory_marker: false,
        })
        .collect::<Vec<_>>();

    Ok(BenchTargets {
        head_commit,
        commit_file_target,
        files_diff_targets,
        files_diff_paths,
    })
}

fn dirty_tracked_paths(config: &LargeRepoConfig) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = text_repo_path(config, 0) {
        paths.push(path);
    }
    if let Some(path) = text_repo_path(config, 1)
        && !paths.contains(&path)
    {
        paths.push(path);
    }
    if let Some(path) = perf_repo::binary_repo_path(config, 0)
        && !paths.contains(&path)
    {
        paths.push(path);
    }
    paths
}

fn select_commit_file_path(
    entries: &[CommitFileEntry],
) -> Result<CommitFileDiffPath, Box<dyn Error>> {
    let selected = entries
        .iter()
        .find(|entry| entry.path.ends_with(".txt"))
        .or_else(|| entries.first())
        .ok_or("HEAD commit has no changed files")?;
    Ok(CommitFileDiffPath {
        path: selected.path.clone(),
        old_path: selected.old_path.clone(),
    })
}

fn measure_record(
    scale: Scale,
    operation: Operation,
    runner: Runner,
    iteration: usize,
    context: &mut MeasureContext<'_>,
) -> PerfRecord {
    let (elapsed, result) = time_result(|| measure_runner(context, operation, runner));
    match result {
        Ok(payload) => PerfRecord {
            scale,
            operation,
            runner,
            iteration,
            elapsed_ms: elapsed.as_millis(),
            output_items: payload.output_items,
            output_bytes: payload.output_bytes,
            success: true,
            error: None,
        },
        Err(error) => PerfRecord {
            scale,
            operation,
            runner,
            iteration,
            elapsed_ms: elapsed.as_millis(),
            output_items: 0,
            output_bytes: 0,
            success: false,
            error: Some(error),
        },
    }
}

fn measure_runner(
    context: &mut MeasureContext<'_>,
    operation: Operation,
    runner: Runner,
) -> Result<MeasurementPayload, String> {
    match runner {
        Runner::GitCliRaw => {
            measure_git_cli_raw(context.git, context.config, context.targets, operation)
        }
        Runner::GitCliParsed => {
            measure_git_cli_parsed(context.git, context.config, context.targets, operation)
        }
        Runner::Backend => measure_backend(context.backend, context.targets, operation),
    }
}

fn measure_git_cli_raw(
    git: &str,
    config: &LargeRepoConfig,
    targets: &BenchTargets,
    operation: Operation,
) -> Result<MeasurementPayload, String> {
    let output = match operation {
        Operation::Status => run_git_output(
            git,
            &config.path,
            status_args(status_mode_for_index_count(config.index_entry_count())),
        )?,
        Operation::Commits => {
            run_git_output(git, &config.path, commit_log_args(0, COMMITS_PAGE_SIZE))?
        }
        Operation::LoadMoreCommits => run_git_output(
            git,
            &config.path,
            commit_log_args(COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE),
        )?,
        Operation::CommitFiles => {
            run_git_output(git, &config.path, commit_files_args(&targets.head_commit))?
        }
        Operation::CommitDetailsDiff => run_git_output(
            git,
            &config.path,
            commit_details_diff_args(&targets.head_commit),
        )?,
        Operation::CommitFileDiff => run_git_output(
            git,
            &config.path,
            commit_file_diff_args(&targets.commit_file_target),
        )?,
        Operation::FilesDetailsDiff => files_details_diff_cli_output(git, &config.path, targets)?,
    };
    Ok(MeasurementPayload {
        output_items: 0,
        output_bytes: output.len(),
    })
}

fn measure_git_cli_parsed(
    git: &str,
    config: &LargeRepoConfig,
    targets: &BenchTargets,
    operation: Operation,
) -> Result<MeasurementPayload, String> {
    match operation {
        Operation::Status => {
            let output = run_git_output(
                git,
                &config.path,
                status_args(status_mode_for_index_count(config.index_entry_count())),
            )?;
            let files = parse_status_porcelain(&output)?;
            Ok(MeasurementPayload {
                output_items: files.len(),
                output_bytes: output.len(),
            })
        }
        Operation::Commits => {
            let output = run_git_output(git, &config.path, commit_log_args(0, COMMITS_PAGE_SIZE))?;
            let items = String::from_utf8_lossy(&output).lines().count();
            Ok(MeasurementPayload {
                output_items: items,
                output_bytes: output.len(),
            })
        }
        Operation::LoadMoreCommits => {
            let output = run_git_output(
                git,
                &config.path,
                commit_log_args(COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE),
            )?;
            let items = String::from_utf8_lossy(&output).lines().count();
            Ok(MeasurementPayload {
                output_items: items,
                output_bytes: output.len(),
            })
        }
        Operation::CommitFiles => {
            let output =
                run_git_text_owned(git, &config.path, commit_files_args(&targets.head_commit))?;
            let entries = parse_commit_files(&output)?;
            Ok(MeasurementPayload {
                output_items: entries.len(),
                output_bytes: output.len(),
            })
        }
        Operation::CommitDetailsDiff => {
            let output = run_git_output(
                git,
                &config.path,
                commit_details_diff_args(&targets.head_commit),
            )?;
            let items = String::from_utf8_lossy(&output).lines().count();
            Ok(MeasurementPayload {
                output_items: items,
                output_bytes: output.len(),
            })
        }
        Operation::CommitFileDiff => {
            let output = run_git_output(
                git,
                &config.path,
                commit_file_diff_args(&targets.commit_file_target),
            )?;
            let items = String::from_utf8_lossy(&output).lines().count();
            Ok(MeasurementPayload {
                output_items: items,
                output_bytes: output.len(),
            })
        }
        Operation::FilesDetailsDiff => {
            let output = files_details_diff_cli_output(git, &config.path, targets)?;
            let items = String::from_utf8_lossy(&output).lines().count();
            Ok(MeasurementPayload {
                output_items: items,
                output_bytes: output.len(),
            })
        }
    }
}

fn measure_backend(
    backend: &mut HybridGitBackend,
    targets: &BenchTargets,
    operation: Operation,
) -> Result<MeasurementPayload, String> {
    match operation {
        Operation::Status => {
            let snapshot = backend.refresh_files().map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: snapshot.files.len(),
                output_bytes: snapshot.files.iter().map(|entry| entry.path.len()).sum(),
            })
        }
        Operation::Commits => {
            let commits = backend.refresh_commits().map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: commits.len(),
                output_bytes: commits.iter().map(|entry| entry.message.len()).sum(),
            })
        }
        Operation::LoadMoreCommits => {
            let commits = backend
                .load_more_commits(COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE)
                .map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: commits.len(),
                output_bytes: commits.iter().map(|entry| entry.message.len()).sum(),
            })
        }
        Operation::CommitFiles => {
            let files = backend
                .commit_files(&targets.head_commit)
                .map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: files.len(),
                output_bytes: files.iter().map(|entry| entry.path.len()).sum(),
            })
        }
        Operation::CommitDetailsDiff => {
            let diff = backend
                .commit_details_diff(&targets.head_commit)
                .map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: diff.lines().count(),
                output_bytes: diff.len(),
            })
        }
        Operation::CommitFileDiff => {
            let diff = backend
                .commit_file_diff(&targets.commit_file_target)
                .map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: diff.lines().count(),
                output_bytes: diff.len(),
            })
        }
        Operation::FilesDetailsDiff => {
            let diff = backend
                .files_details_diff(&targets.files_diff_targets)
                .map_err(|error| error.message)?;
            Ok(MeasurementPayload {
                output_items: diff.lines().count(),
                output_bytes: diff.len(),
            })
        }
    }
}

fn time_result<T>(operation: impl FnOnce() -> Result<T, String>) -> (Duration, Result<T, String>) {
    let started = Instant::now();
    let result = operation();
    (started.elapsed(), result)
}

fn status_mode_for_index_count(index_entry_count: usize) -> StatusModeForPerf {
    if index_entry_count >= 1_000_000 {
        StatusModeForPerf::HugeRepoMetadataOnly
    } else if index_entry_count >= 100_000 {
        StatusModeForPerf::LargeRepoFast
    } else {
        StatusModeForPerf::Full
    }
}

fn status_args(mode: StatusModeForPerf) -> Vec<String> {
    vec![
        "status".to_string(),
        "--porcelain=v1".to_string(),
        "-z".to_string(),
        match mode {
            StatusModeForPerf::Full => "--untracked-files=all".to_string(),
            StatusModeForPerf::LargeRepoFast | StatusModeForPerf::HugeRepoMetadataOnly => {
                "--untracked-files=no".to_string()
            }
        },
        "--ignored=no".to_string(),
        "--ignore-submodules=all".to_string(),
    ]
}

fn commit_log_args(skip: usize, limit: usize) -> Vec<String> {
    vec![
        "log".to_string(),
        format!("--skip={skip}"),
        "-n".to_string(),
        limit.to_string(),
        "--format=%H%x09%h%x09%an%x09%s".to_string(),
    ]
}

fn commit_files_args(commit_id: &str) -> Vec<String> {
    vec![
        "diff-tree".to_string(),
        "--root".to_string(),
        "--no-commit-id".to_string(),
        "--name-status".to_string(),
        "-r".to_string(),
        "-M".to_string(),
        "-C".to_string(),
        commit_id.to_string(),
    ]
}

fn commit_details_diff_args(commit_id: &str) -> Vec<String> {
    vec![
        "show".to_string(),
        "--no-color".to_string(),
        "--no-ext-diff".to_string(),
        "--no-textconv".to_string(),
        "--no-renames".to_string(),
        "--format=fuller".to_string(),
        "--patch".to_string(),
        commit_id.to_string(),
    ]
}

fn commit_file_diff_args(target: &CommitFileDiffTarget) -> Vec<String> {
    let mut args = vec![
        "show".to_string(),
        "--no-color".to_string(),
        "--format=".to_string(),
        "--patch".to_string(),
        "--find-renames".to_string(),
        "--find-copies".to_string(),
        target.commit_id.clone(),
        "--".to_string(),
    ];
    for path in &target.paths {
        if let Some(old_path) = &path.old_path {
            args.push(old_path.clone());
        }
        args.push(path.path.clone());
    }
    args
}

fn files_details_diff_cli_output(
    git: &str,
    repo: &Path,
    targets: &BenchTargets,
) -> Result<Vec<u8>, String> {
    let mut unstaged_args = vec![
        "diff".to_string(),
        "--no-color".to_string(),
        "--no-ext-diff".to_string(),
        "--no-textconv".to_string(),
        "--".to_string(),
    ];
    unstaged_args.extend(targets.files_diff_paths.iter().cloned());
    let mut output = run_git_output(git, repo, unstaged_args)?;

    let mut staged_args = vec![
        "diff".to_string(),
        "--cached".to_string(),
        "--no-color".to_string(),
        "--no-ext-diff".to_string(),
        "--no-textconv".to_string(),
        "--".to_string(),
    ];
    staged_args.extend(targets.files_diff_paths.iter().cloned());
    output.extend(run_git_output(git, repo, staged_args)?);
    Ok(output)
}

fn parse_status_porcelain(output: &[u8]) -> Result<Vec<FileEntry>, String> {
    let mut entries = Vec::new();
    let mut records = output.split(|byte| *byte == 0).peekable();
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        if record.len() < 4 || record[2] != b' ' {
            return Err("invalid git status porcelain record".to_string());
        }
        let index = record[0];
        let worktree = record[1];
        if index == b'!' && worktree == b'!' {
            continue;
        }
        let path = String::from_utf8(record[3..].to_vec())
            .map_err(|error| format!("invalid utf-8 path from git status: {error}"))?
            .replace('\\', "/");
        if matches!(index, b'R' | b'C') || matches!(worktree, b'R' | b'C') {
            let _ = records.next();
        }
        entries.push(FileEntry {
            path,
            staged: matches!(index, b'A' | b'M' | b'D' | b'R' | b'C' | b'T' | b'U'),
            untracked: index == b'?' && worktree == b'?',
        });
    }
    Ok(entries)
}

fn parse_commit_files(output: &str) -> Result<Vec<CommitFileEntry>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_commit_file_line)
        .collect()
}

fn parse_commit_file_line(line: &str) -> Result<CommitFileEntry, String> {
    let parts = line.split('\t').collect::<Vec<_>>();
    let raw_status = parts
        .first()
        .ok_or_else(|| "missing commit file status".to_string())?;
    let status = match raw_status.chars().next().unwrap_or('?') {
        'A' => CommitFileStatus::Added,
        'M' => CommitFileStatus::Modified,
        'D' => CommitFileStatus::Deleted,
        'R' => CommitFileStatus::Renamed,
        'C' => CommitFileStatus::Copied,
        'T' => CommitFileStatus::TypeChanged,
        _ => CommitFileStatus::Unknown,
    };
    let (old_path, path) = match status {
        CommitFileStatus::Renamed | CommitFileStatus::Copied => {
            if parts.len() < 3 {
                return Err(format!("invalid commit file line: {line}"));
            }
            (Some(parts[1].to_string()), parts[2].to_string())
        }
        _ => {
            if parts.len() < 2 {
                return Err(format!("invalid commit file line: {line}"));
            }
            (None, parts[1].to_string())
        }
    };
    Ok(CommitFileEntry {
        path,
        old_path,
        status,
    })
}

fn run_git_output(git: &str, repo: &Path, args: Vec<String>) -> Result<Vec<u8>, String> {
    let output = Command::new(git)
        .args(&args)
        .current_dir(repo)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|error| format!("failed to start git {:?}: {error}", args))?;
    if !output.status.success() {
        return Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn run_git_text<I, S>(git: &str, repo: &Path, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string())
        .collect::<Vec<_>>();
    run_git_text_owned(git, repo, args)
}

fn run_git_text_owned(git: &str, repo: &Path, args: Vec<String>) -> Result<String, String> {
    run_git_output(git, repo, args).map(|stdout| String::from_utf8_lossy(&stdout).to_string())
}

fn run_git_status<I, S>(git: &str, repo: &Path, args: I) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    run_git_text(git, repo, args)
        .map(|_| ())
        .map_err(Into::into)
}

fn git_version(git: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new(git).arg("--version").output()?;
    if !output.status.success() {
        return Err("git --version failed".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch");
    format!("perf-{}-{:09}", now.as_secs(), now.subsec_nanos())
}

fn write_json_report(report: &SuiteReport) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(&report.json_path)?;
    writeln!(file, "{{")?;
    writeln!(file, "  \"schema\": \"ratagit.perf-suite.v1\",")?;
    writeln!(file, "  \"run_id\": {},", json_string(&report.run_id))?;
    writeln!(
        file,
        "  \"git_version\": {},",
        json_string(&report.git_version)
    )?;
    writeln!(file, "  \"repos\": [")?;
    for (index, repo) in report.repos.iter().enumerate() {
        writeln!(file, "    {{")?;
        writeln!(
            file,
            "      \"scale\": {},",
            json_string(repo.scale.as_str())
        )?;
        writeln!(
            file,
            "      \"path\": {},",
            json_string(&repo.path.display().to_string())
        )?;
        writeln!(file, "      \"reused\": {},", repo.reused)?;
        writeln!(file, "      \"files\": {},", repo.manifest.files)?;
        writeln!(file, "      \"text_files\": {},", repo.manifest.text_files)?;
        writeln!(
            file,
            "      \"binary_files\": {},",
            repo.manifest.binary_files
        )?;
        writeln!(file, "      \"commits\": {}", repo.manifest.commits)?;
        write!(file, "    }}")?;
        if index + 1 != report.repos.len() {
            writeln!(file, ",")?;
        } else {
            writeln!(file)?;
        }
    }
    writeln!(file, "  ],")?;
    writeln!(file, "  \"summaries\": [")?;
    let summaries = summary_rows(report);
    for (index, summary) in summaries.iter().enumerate() {
        writeln!(file, "    {{")?;
        writeln!(
            file,
            "      \"scale\": {},",
            json_string(summary.scale.as_str())
        )?;
        writeln!(
            file,
            "      \"operation\": {},",
            json_string(summary.operation.as_str())
        )?;
        write!(file, "      \"git_cli_raw_ms\": ")?;
        write_stats_json(file.by_ref(), summary.raw)?;
        writeln!(file, ",")?;
        write!(file, "      \"git_cli_parsed_ms\": ")?;
        write_stats_json(file.by_ref(), summary.parsed)?;
        writeln!(file, ",")?;
        write!(file, "      \"backend_ms\": ")?;
        write_stats_json(file.by_ref(), summary.backend)?;
        writeln!(file, ",")?;
        writeln!(
            file,
            "      \"backend_vs_cli_raw_ratio\": {},",
            json_optional_f64(ratio_value(summary.backend, summary.raw))
        )?;
        writeln!(
            file,
            "      \"backend_vs_cli_parsed_ratio\": {}",
            json_optional_f64(ratio_value(summary.backend, summary.parsed))
        )?;
        write!(file, "    }}")?;
        if index + 1 != summaries.len() {
            writeln!(file, ",")?;
        } else {
            writeln!(file)?;
        }
    }
    writeln!(file, "  ],")?;
    writeln!(file, "  \"records\": [")?;
    for (index, record) in report.records.iter().enumerate() {
        writeln!(file, "    {{")?;
        writeln!(
            file,
            "      \"scale\": {},",
            json_string(record.scale.as_str())
        )?;
        writeln!(
            file,
            "      \"operation\": {},",
            json_string(record.operation.as_str())
        )?;
        writeln!(
            file,
            "      \"runner\": {},",
            json_string(record.runner.as_str())
        )?;
        writeln!(file, "      \"iteration\": {},", record.iteration)?;
        writeln!(file, "      \"elapsed_ms\": {},", record.elapsed_ms)?;
        writeln!(file, "      \"output_items\": {},", record.output_items)?;
        writeln!(file, "      \"output_bytes\": {},", record.output_bytes)?;
        writeln!(file, "      \"success\": {},", record.success)?;
        write!(file, "      \"error\": ")?;
        if let Some(error) = &record.error {
            writeln!(file, "{}", json_string(error))?;
        } else {
            writeln!(file, "null")?;
        }
        write!(file, "    }}")?;
        if index + 1 != report.records.len() {
            writeln!(file, ",")?;
        } else {
            writeln!(file)?;
        }
    }
    writeln!(file, "  ]")?;
    writeln!(file, "}}")?;
    Ok(())
}

fn write_markdown_report(report: &SuiteReport) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(&report.markdown_path)?;
    writeln!(file, "# ratagit performance suite")?;
    writeln!(file)?;
    writeln!(file, "- run: `{}`", report.run_id)?;
    writeln!(file, "- git: `{}`", report.git_version)?;
    writeln!(file)?;
    writeln!(file, "## Summary")?;
    writeln!(file)?;
    writeln!(
        file,
        "| scale | operation | raw ms min/med/max | parsed ms min/med/max | backend ms min/med/max | backend/raw | backend/parsed |"
    )?;
    writeln!(file, "| --- | --- | ---: | ---: | ---: | ---: | ---: |")?;

    for summary in summary_rows(report) {
        writeln!(
            file,
            "| {} | {} | {} | {} | {} | {} | {} |",
            summary.scale.as_str(),
            summary.operation.as_str(),
            format_optional_stats(summary.raw),
            format_optional_stats(summary.parsed),
            format_optional_stats(summary.backend),
            format_ratio(ratio_value(summary.backend, summary.raw)),
            format_ratio(ratio_value(summary.backend, summary.parsed)),
        )?;
    }

    let failures = report
        .records
        .iter()
        .filter(|record| !record.success)
        .collect::<Vec<_>>();
    if !failures.is_empty() {
        writeln!(file)?;
        writeln!(file, "## Failures")?;
        writeln!(file)?;
        writeln!(file, "| scale | operation | runner | iteration | error |")?;
        writeln!(file, "| --- | --- | --- | ---: | --- |")?;
        for failure in failures {
            writeln!(
                file,
                "| {} | {} | {} | {} | {} |",
                failure.scale.as_str(),
                failure.operation.as_str(),
                failure.runner.as_str(),
                failure.iteration,
                failure
                    .error
                    .as_deref()
                    .unwrap_or("unknown")
                    .replace('|', "\\|")
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stats {
    min_ms: u128,
    median_ms: u128,
    max_ms: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SummaryRow {
    scale: Scale,
    operation: Operation,
    raw: Option<Stats>,
    parsed: Option<Stats>,
    backend: Option<Stats>,
}

fn summary_rows(report: &SuiteReport) -> Vec<SummaryRow> {
    let mut rows = Vec::new();
    for scale in report.repos.iter().map(|repo| repo.scale) {
        for operation in Operation::ALL {
            let raw = stats_for(report, scale, operation, Runner::GitCliRaw);
            let parsed = stats_for(report, scale, operation, Runner::GitCliParsed);
            let backend = stats_for(report, scale, operation, Runner::Backend);
            if raw.is_some() || parsed.is_some() || backend.is_some() {
                rows.push(SummaryRow {
                    scale,
                    operation,
                    raw,
                    parsed,
                    backend,
                });
            }
        }
    }
    rows
}

fn stats_for(
    report: &SuiteReport,
    scale: Scale,
    operation: Operation,
    runner: Runner,
) -> Option<Stats> {
    let mut values = report
        .records
        .iter()
        .filter(|record| {
            record.scale == scale
                && record.operation == operation
                && record.runner == runner
                && record.success
        })
        .map(|record| record.elapsed_ms)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(Stats {
        min_ms: values[0],
        median_ms: values[values.len() / 2],
        max_ms: values[values.len() - 1],
    })
}

fn format_optional_stats(stats: Option<Stats>) -> String {
    stats.map_or_else(
        || "-".to_string(),
        |stats| format!("{}/{}/{}", stats.min_ms, stats.median_ms, stats.max_ms),
    )
}

fn ratio_value(numerator: Option<Stats>, denominator: Option<Stats>) -> Option<f64> {
    match (numerator, denominator) {
        (Some(numerator), Some(denominator)) if denominator.median_ms > 0 => {
            Some(numerator.median_ms as f64 / denominator.median_ms as f64)
        }
        (Some(numerator), Some(denominator))
            if numerator.median_ms == 0 && denominator.median_ms == 0 =>
        {
            Some(1.0)
        }
        _ => None,
    }
}

fn format_ratio(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |value| format!("{value:.2}"))
}

fn json_optional_f64(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_string(), |value| format!("{value:.4}"))
}

fn write_stats_json(mut writer: impl Write, stats: Option<Stats>) -> Result<(), Box<dyn Error>> {
    if let Some(stats) = stats {
        write!(
            writer,
            "{{\"min\": {}, \"median\": {}, \"max\": {}}}",
            stats.min_ms, stats.median_ms, stats.max_ms
        )?;
    } else {
        write!(writer, "null")?;
    }
    Ok(())
}

fn parse_scales(value: &str) -> Result<Vec<Scale>, PerfConfigError> {
    parse_csv(value, |part| {
        Scale::parse(part).map_err(|_| PerfConfigError::InvalidValue("unknown scale"))
    })
}

fn parse_operations(value: &str) -> Result<Vec<Operation>, PerfConfigError> {
    parse_csv(value, Operation::parse)
}

fn parse_csv<T>(
    value: &str,
    mut parse: impl FnMut(&str) -> Result<T, PerfConfigError>,
) -> Result<Vec<T>, PerfConfigError> {
    let mut items = Vec::new();
    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        items.push(parse(part)?);
    }
    Ok(items)
}

fn value_after<'a>(
    args: &'a [String],
    index: usize,
    option: &'static str,
) -> Result<&'a str, PerfConfigError> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or(PerfConfigError::MissingValue(option))
}

fn parse_usize(value: &str, option: &'static str) -> Result<usize, PerfConfigError> {
    value.parse().map_err(|_| PerfConfigError::InvalidNumber {
        option,
        value: value.to_string(),
    })
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", other as u32));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

fn perf_suite_usage() -> &'static str {
    "Usage: cargo run --bin perf-suite -- [options]\n\
\n\
Options:\n\
  --root <path>          Synthetic repo root (default: tmp/perf/suite)\n\
  --output <path>        Report output directory (default: tmp/perf/results)\n\
  --scales <list>        Comma-separated scales (default: smoke,small,medium,large)\n\
  --operations <list>    Comma-separated operations to run\n\
  --iterations <count>   Measured iterations per runner (default: 3)\n\
  --warmup <count>       Warmup iterations per runner (default: 1)\n\
  --git <path>           Git executable (default: git)\n\
  --regenerate           Rebuild synthetic repos that already exist\n\
  -h, --help             Print this help text"
}

#[cfg(test)]
mod tests {
    use std::fs::remove_dir_all;

    use super::*;

    #[test]
    fn parses_defaults() {
        let config = PerfConfig::parse(Vec::new()).expect("default config should parse");

        assert_eq!(config.root, PathBuf::from("tmp/perf/suite"));
        assert_eq!(config.output, PathBuf::from("tmp/perf/results"));
        assert_eq!(config.scales, Scale::DEFAULT_SUITE.to_vec());
        assert_eq!(config.operations, Operation::ALL.to_vec());
        assert_eq!(config.iterations, 3);
        assert_eq!(config.warmup, 1);
        assert!(!config.regenerate);
    }

    #[test]
    fn parses_explicit_huge_and_operation_filters() {
        let config = PerfConfig::parse(vec![
            "--root".into(),
            "tmp/custom-suite".into(),
            "--output".into(),
            "tmp/custom-results".into(),
            "--scales".into(),
            "smoke,huge".into(),
            "--operations".into(),
            "status,commit-files".into(),
            "--iterations".into(),
            "2".into(),
            "--warmup".into(),
            "0".into(),
            "--git".into(),
            "custom-git".into(),
            "--regenerate".into(),
        ])
        .expect("explicit config should parse");

        assert_eq!(config.root, PathBuf::from("tmp/custom-suite"));
        assert_eq!(config.output, PathBuf::from("tmp/custom-results"));
        assert_eq!(config.scales, vec![Scale::Smoke, Scale::Huge]);
        assert_eq!(
            config.operations,
            vec![Operation::Status, Operation::CommitFiles]
        );
        assert_eq!(config.iterations, 2);
        assert_eq!(config.warmup, 0);
        assert_eq!(config.git, "custom-git");
        assert!(config.regenerate);
    }

    #[test]
    fn status_args_match_backend_modes() {
        assert!(
            status_args(StatusModeForPerf::Full).contains(&"--untracked-files=all".to_string())
        );
        assert!(
            status_args(StatusModeForPerf::LargeRepoFast)
                .contains(&"--untracked-files=no".to_string())
        );
        assert!(
            status_args(StatusModeForPerf::HugeRepoMetadataOnly)
                .contains(&"--untracked-files=no".to_string())
        );
    }

    #[test]
    fn commit_file_args_are_stable() {
        let args = commit_files_args("abc123");

        assert_eq!(
            args,
            vec![
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-status",
                "-r",
                "-M",
                "-C",
                "abc123"
            ]
        );
    }

    #[test]
    fn parses_status_and_commit_files_baselines() {
        let status =
            parse_status_porcelain(b" M a.txt\0A  b.txt\0?? c.txt\0").expect("status should parse");

        assert_eq!(status.len(), 3);
        assert!(!status[0].staged);
        assert!(status[1].staged);
        assert!(status[2].untracked);

        let files = parse_commit_files("A\tnew.txt\nR100\told.txt\trenamed.txt\n")
            .expect("commit files should parse");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].status, CommitFileStatus::Added);
        assert_eq!(files[1].old_path.as_deref(), Some("old.txt"));
        assert_eq!(files[1].path, "renamed.txt");
    }

    #[test]
    fn markdown_stats_compute_backend_ratios() {
        let report = SuiteReport {
            run_id: "test".to_string(),
            git_version: "git version test".to_string(),
            repos: Vec::new(),
            records: vec![
                test_record(Runner::GitCliRaw, 10),
                test_record(Runner::GitCliRaw, 20),
                test_record(Runner::GitCliParsed, 15),
                test_record(Runner::GitCliParsed, 25),
                test_record(Runner::Backend, 30),
                test_record(Runner::Backend, 40),
            ],
            json_path: PathBuf::new(),
            markdown_path: PathBuf::new(),
        };

        assert_eq!(
            stats_for(&report, Scale::Smoke, Operation::Status, Runner::Backend),
            Some(Stats {
                min_ms: 30,
                median_ms: 40,
                max_ms: 40
            })
        );
        assert_eq!(
            format_ratio(ratio_value(
                Some(Stats {
                    min_ms: 30,
                    median_ms: 40,
                    max_ms: 40
                }),
                Some(Stats {
                    min_ms: 10,
                    median_ms: 20,
                    max_ms: 20
                })
            )),
            "2.00"
        );
    }

    #[test]
    fn smoke_suite_generates_reports_with_all_runners() {
        let git_available = Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        if !git_available {
            return;
        }

        let root = workspace_tmp_root().join("perf-suite-smoke");
        let output = workspace_tmp_root().join("perf-suite-results");
        let _ = remove_dir_all(&root);
        let _ = remove_dir_all(&output);

        let config = PerfConfig {
            root: root.clone(),
            output: output.clone(),
            scales: vec![Scale::Smoke],
            operations: vec![Operation::Status, Operation::CommitFiles],
            iterations: 1,
            warmup: 0,
            git: "git".to_string(),
            regenerate: false,
            help: false,
        };

        let report = run_suite(&config).expect("smoke suite should run");

        assert!(report.json_path.is_file());
        assert!(report.markdown_path.is_file());
        for runner in Runner::ALL {
            assert!(
                report
                    .records
                    .iter()
                    .any(|record| record.runner == runner && record.success)
            );
        }

        let _ = remove_dir_all(root);
        let _ = remove_dir_all(output);
    }

    fn test_record(runner: Runner, elapsed_ms: u128) -> PerfRecord {
        PerfRecord {
            scale: Scale::Smoke,
            operation: Operation::Status,
            runner,
            iteration: 0,
            elapsed_ms,
            output_items: 0,
            output_bytes: 0,
            success: true,
            error: None,
        }
    }

    fn workspace_tmp_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tmp")
    }
}
