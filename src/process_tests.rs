//! Real child processes isolate libgit2, config lookup, environment and workers.
#[path = "../tests/support/mod.rs"]
pub(crate) mod support;

use crate::config::keymap::Keymap;
use crate::git::patch::{HunkId, Patch, PatchDirection, PatchRequest};
use crate::git::{
    Git2Repository, GitError, GitErrorCategory, HistoryRepository, PatchRepository,
    ReferenceRepository, WorktreeRepository,
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use support::ProcessFixture;

fn child_command(fixture: &ProcessFixture, scenario: &str) -> std::process::Command {
    let mut command = fixture.command(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "process_tests::test_child_entry",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RATAGIT_TEST_SCENARIO", scenario)
        .env("RATAGIT_TEST_ROOT", fixture.path(""));
    command
}

fn report(fixture: &ProcessFixture, command: &mut std::process::Command) -> serde_json::Value {
    let output = fixture.run(command);
    let stdout = fixture.normalize(&output.stdout);
    let stderr = fixture.normalize(&output.stderr);
    assert!(
        output.status.success(),
        "status={} stdout={stdout} stderr={stderr}",
        output.status
    );
    assert!(stderr.is_empty(), "unexpected child diagnostics: {stderr}");
    assert!(
        !stdout.contains('\x1b'),
        "unexpected terminal output: {stdout}"
    );
    let marker = "RATAGIT_PROCESS_REPORT ";
    let start = stdout
        .find(marker)
        .expect("child scenario must actually execute")
        + marker.len();
    serde_json::from_str(stdout[start..].lines().next().unwrap()).unwrap()
}

#[test]
fn test_process_configuration_recovery_and_crlf_bindings_are_isolated() {
    let fixture = ProcessFixture::new();
    let original = std::fs::read(fixture.path("home/.gitconfig")).unwrap();
    let parent_home = std::env::var_os("HOME");
    let result = report(&fixture, &mut child_command(&fixture, "config"));
    assert_eq!(result["config"], "recovered and loaded CRLF bindings");
    assert_eq!(std::env::var_os("HOME"), parent_home);
    assert_eq!(
        std::fs::read(fixture.path("home/.gitconfig")).unwrap(),
        original
    );
    assert!(
        std::fs::read_to_string(fixture.path("config/ratagit/keymap.toml"))
            .unwrap()
            .contains("\r\n")
    );
}

#[test]
fn test_process_fixture_removes_hostile_git_and_app_overrides() {
    let fixture = ProcessFixture::new();
    let mut command = child_command(&fixture, "isolation");
    command
        .env("GIT_DIR", "wrong-repository")
        .env("GIT_CONFIG_PARAMETERS", "invalid")
        .env("GIT_INDEX_FILE", "wrong-index")
        .env("RATAGIT_UPDATE_GOLDEN", "1")
        .env("HOME", "wrong-home");
    fixture.isolate(&mut command);
    command
        .env("RATAGIT_TEST_SCENARIO", "isolation")
        .env("RATAGIT_TEST_ROOT", fixture.path(""));
    let result = report(&fixture, &mut command);
    assert_eq!(result["identity"], "Process Fixture");
    assert_eq!(result["branch"], "main");
}

#[test]
fn test_process_unicode_paths_and_crlf_hunks_roundtrip() {
    let fixture = ProcessFixture::new();
    let result = report(&fixture, &mut child_command(&fixture, "patch"));
    assert_eq!(result["path"], "目录/naïve 文件.txt");
    assert_eq!(result["separator"], std::path::MAIN_SEPARATOR.to_string());
    assert_eq!(result["crlf"], "preserved through stage and unstage");
}

#[test]
fn test_process_diagnostics_normalize_root_but_retain_path_code_and_class() {
    let fixture = ProcessFixture::new();
    let output = fixture.run(&mut child_command(&fixture, "diagnostic"));
    assert!(output.status.success());
    let stderr = fixture.normalize(&output.stderr);
    assert!(stderr.contains("<ROOT>"), "{stderr}");
    assert!(!stderr.contains(fixture.path("").to_str().unwrap()));
    assert!(stderr.contains("missing 文件.txt"), "{stderr}");
    assert!(
        stderr.contains("NotFound") && stderr.contains("code -3") && stderr.contains("class "),
        "{stderr}"
    );
    assert!(!stderr.contains('\x1b'));
}

#[test]
fn test_process_workers_drain_queued_results_and_exit_without_live_repository_handles() {
    let fixture = ProcessFixture::new();
    let result = report(&fixture, &mut child_command(&fixture, "shutdown"));
    assert_eq!(result["workers"], "drained");
    // On Windows, a surviving repository handle can prevent this rename.
    std::fs::rename(&fixture.work, fixture.path("released tree")).unwrap();
}

#[test]
fn test_process_graph_environment_is_independent_between_children() {
    for (value, expected) in [(None, "◯"), (Some("1"), "*"), (Some("0"), "◯")] {
        let fixture = ProcessFixture::new();
        let mut command = child_command(&fixture, "graph");
        if let Some(value) = value {
            command.env("RATAGIT_GRAPH_ASCII", value);
        }
        let result = report(&fixture, &mut command);
        assert_eq!(result["node"], expected);
    }
}

#[test]
fn test_process_editor_precedence_arguments_and_exit_status_are_isolated() {
    for override_editor in [false, true] {
        let fixture = ProcessFixture::new();
        let mut command = child_command(&fixture, "editor");
        if override_editor {
            command.env("GIT_EDITOR", "git -c core.quotePath=false hash-object --");
        }
        let result = report(&fixture, &mut command);
        assert_eq!(result["override"], override_editor);
        assert_eq!(result["editor"], "completed with literal Unicode path");
    }
}

fn initialize_repository(work: &Path) {
    let repo = git2::Repository::init(work).unwrap();
    assert_eq!(
        repo.config().unwrap().get_string("user.name").unwrap(),
        "Process Fixture"
    );
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.filemode", false)
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    std::fs::write(work.join(".gitattributes"), "* -text\n").unwrap();
    std::fs::create_dir(work.join("目录")).unwrap();
    std::fs::write(work.join("目录/naïve 文件.txt"), b"one\r\ntwo\r\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".gitattributes")).unwrap();
    index.add_path(Path::new("目录/naïve 文件.txt")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::new(
        "Process Fixture",
        "fixture@example.invalid",
        &git2::Time::new(946684800, 0),
    )
    .unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[])
        .unwrap();
}

fn ids(patch: &Patch) -> Vec<HunkId> {
    patch
        .files
        .iter()
        .enumerate()
        .flat_map(|(file, entry)| (0..entry.hunks.len()).map(move |hunk| HunkId { file, hunk }))
        .collect()
}

fn indexed_bytes(work: &Path, path: &Path) -> Vec<u8> {
    let repo = git2::Repository::open(work).unwrap();
    let entry = repo.index().unwrap().get_path(path, 0).unwrap();
    let bytes = repo.find_blob(entry.id).unwrap().content().to_vec();
    bytes
}

#[test]
#[ignore = "entry point invoked only by the isolated subprocess tests"]
fn test_child_entry() {
    let root = PathBuf::from(std::env::var_os("RATAGIT_TEST_ROOT").expect("fixture root"));
    // SAFETY: this child runs exactly one test. Set libgit2's search paths before
    // opening repositories or starting workers. No other thread uses libgit2.
    for level in [
        git2::ConfigLevel::System,
        git2::ConfigLevel::Global,
        git2::ConfigLevel::XDG,
        git2::ConfigLevel::ProgramData,
    ] {
        unsafe {
            git2::opts::set_search_path(level, root.join("home")).unwrap();
        }
    }
    let work = std::env::current_dir().unwrap();
    let scenario = std::env::var("RATAGIT_TEST_SCENARIO").unwrap();
    let result = match scenario.as_str() {
        "config" => {
            let path = root.join("config/ratagit/keymap.toml");
            let defaults = Keymap::load_from_path(&path);
            assert!(defaults.global_keys("quit").contains(&"q"));
            std::fs::write(&path, "[global\ninvalid").unwrap();
            let recovered = Keymap::load_from_path(&path);
            assert!(recovered.global_keys("quit").contains(&"q"));
            toml::from_str::<Keymap>(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let custom = "[global]\r\nquit = [\"x\"]\r\n[files]\r\nstage = [\"a\"]\r\n";
            std::fs::write(&path, custom).unwrap();
            let loaded = Keymap::load_from_path(&path);
            assert_eq!(loaded.global_keys("quit"), ["x"]);
            assert_eq!(loaded.panel_keys("files", "stage"), ["a"]);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), custom);
            // Failed persistence must keep the established silent fallback.
            let blocked = root.join("config/blocked.toml");
            std::fs::create_dir(&blocked).unwrap();
            assert!(Keymap::load_from_path(&blocked)
                .global_keys("quit")
                .contains(&"q"));
            serde_json::json!({"config": "recovered and loaded CRLF bindings"})
        }
        "isolation" => {
            for key in [
                "GIT_DIR",
                "GIT_WORK_TREE",
                "GIT_INDEX_FILE",
                "GIT_CONFIG_PARAMETERS",
                "RATAGIT_UPDATE_GOLDEN",
                "RATAGIT_GRAPH_ASCII",
            ] {
                assert!(std::env::var_os(key).is_none(), "inherited {key}");
            }
            assert_eq!(
                PathBuf::from(std::env::var_os("HOME").unwrap()),
                root.join("home")
            );
            let config = git2::Config::open_default().unwrap();
            let git = std::process::Command::new("git")
                .args(["config", "--global", "user.name"])
                .output()
                .unwrap();
            assert!(git.status.success());
            assert_eq!(
                String::from_utf8(git.stdout).unwrap().trim(),
                "Process Fixture"
            );
            serde_json::json!({"identity": config.get_string("user.name").unwrap(), "branch": config.get_string("init.defaultbranch").unwrap()})
        }
        _ => {
            initialize_repository(&work);
            let repo = Git2Repository::open(&work).unwrap();
            match scenario.as_str() {
                "patch" => {
                    let relative = Path::new("目录").join("naïve 文件.txt");
                    std::fs::write(work.join(&relative), b"one\r\nchanged\r\n").unwrap();
                    let patch = repo
                        .patch(PatchRequest::Worktree(relative.clone()))
                        .unwrap()
                        .unwrap();
                    assert!(!ids(&patch).is_empty());
                    assert_eq!(patch.files[0].new.path.as_ref(), Some(&relative));
                    repo.apply_hunks(&patch, &ids(&patch), PatchDirection::Stage)
                        .unwrap();
                    assert_eq!(indexed_bytes(&work, &relative), b"one\r\nchanged\r\n");
                    let patch = repo
                        .patch(PatchRequest::Index(relative.clone()))
                        .unwrap()
                        .unwrap();
                    repo.apply_hunks(&patch, &ids(&patch), PatchDirection::Unstage)
                        .unwrap();
                    assert_eq!(indexed_bytes(&work, &relative), b"one\r\ntwo\r\n");
                    assert_eq!(
                        std::fs::read(work.join(&relative)).unwrap(),
                        b"one\r\nchanged\r\n"
                    );
                    #[cfg(unix)]
                    {
                        let literal = Path::new("literal\\name.txt");
                        std::fs::write(work.join(literal), "literal backslash").unwrap();
                        assert!(repo
                            .status()
                            .unwrap()
                            .untracked
                            .iter()
                            .any(|file| file.path == literal));
                    }
                    #[cfg(windows)]
                    assert_eq!(Path::new("目录\\naïve 文件.txt"), relative);
                    serde_json::json!({"path": "目录/naïve 文件.txt", "separator": std::path::MAIN_SEPARATOR.to_string(), "crlf": "preserved through stage and unstage"})
                }
                "editor" => {
                    let raw = git2::Repository::open(&work).unwrap();
                    raw.config()
                        .unwrap()
                        .set_str("core.editor", "git hash-object --")
                        .unwrap();
                    let relative = Path::new("目录/editor 文件 &.txt");
                    std::fs::write(work.join(relative), "editor fixture\n").unwrap();
                    let prepared = repo.prepare_editor(relative).unwrap();
                    let has_override = std::env::var_os("GIT_EDITOR").is_some();
                    assert_eq!(prepared.program, "git");
                    assert_eq!(prepared.cwd, work);
                    assert_eq!(
                        Path::new(prepared.args.last().unwrap()),
                        work.join(relative)
                    );
                    assert_eq!(prepared.args.len(), if has_override { 5 } else { 3 });
                    prepared.run().unwrap();
                    std::fs::write(work.join("目录/naïve 文件.txt"), "changed\n").unwrap();
                    let nonzero = crate::desktop::EditorCommand {
                        program: "git".into(),
                        args: vec![
                            "-c".into(),
                            "color.ui=false".into(),
                            "diff".into(),
                            "--exit-code".into(),
                            "--".into(),
                            work.join("目录/naïve 文件.txt").to_str().unwrap().into(),
                        ],
                        cwd: work.clone(),
                    };
                    assert!(nonzero
                        .run()
                        .unwrap_err()
                        .to_string()
                        .contains("editor exited"));
                    let missing = crate::desktop::EditorCommand {
                        program: work.join("nonexistent-editor").to_string_lossy().into(),
                        args: vec![],
                        cwd: work.clone(),
                    };
                    assert!(missing.run().is_err());
                    serde_json::json!({"override": has_override, "editor": "completed with literal Unicode path"})
                }
                "diagnostic" => {
                    let error = repo.checkout_branch("missing-branch").unwrap_err();
                    assert_eq!(error.category(), GitErrorCategory::NotFound);
                    eprintln!(
                        "path={}; category={:?}; {error}",
                        work.join("missing 文件.txt").display(),
                        error.category()
                    );
                    assert!(matches!(error, GitError::Context { .. }));
                    serde_json::json!({"diagnostic": "preserved"})
                }
                "shutdown" => {
                    let status = repo.status_async().unwrap();
                    let branches = repo.branches_async().unwrap();
                    let history = repo.commits_async(10).unwrap();
                    let fetch = repo.fetch_default_async().unwrap();
                    drop(repo);
                    let timeout = Duration::from_secs(10);
                    status.recv_timeout(timeout).unwrap().unwrap();
                    assert!(!branches.recv_timeout(timeout).unwrap().unwrap().is_empty());
                    assert_eq!(history.recv_timeout(timeout).unwrap().unwrap().len(), 1);
                    assert_eq!(
                        fetch.recv_timeout(timeout).unwrap().unwrap_err().category(),
                        GitErrorCategory::NotFound
                    );
                    serde_json::json!({"workers": "drained"})
                }
                "graph" => {
                    let commits = repo.commits(10).unwrap();
                    let node = commits[0]
                        .graph
                        .iter()
                        .flat_map(|cell| cell.text.chars())
                        .find(|ch| *ch == '*' || *ch == '◯')
                        .unwrap();
                    serde_json::json!({"node": node.to_string()})
                }
                _ => panic!("unknown process scenario {scenario}"),
            }
        }
    };
    println!("RATAGIT_PROCESS_REPORT {result}");
}
