use super::*;
use ratagit_core::{RepositoryItem as I, RepositoryTab as T};

#[test]
fn repository_tabs_read_real_refs_worktrees_submodules_and_reflog_without_mutation() {
    let repo = TmpGitRepo::new("repository-tabs");
    write(repo.path().join("tracked.txt"), "initial\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "initial"]);
    let head = repo
        .run_git_capture(&["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    repo.run_git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/repo.git",
    ]);
    repo.run_git(&[
        "remote",
        "set-url",
        "--push",
        "origin",
        "ssh://example.invalid/repo.git",
    ]);
    repo.run_git(&["update-ref", "refs/remotes/origin/main", &head]);
    repo.run_git(&[
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
    ]);
    repo.run_git(&["remote", "add", "team", "https://example.invalid/team.git"]);
    // Legacy/manual configuration may contain overlapping remote names.
    repo.run_git(&[
        "config",
        "remote.team/service.url",
        "https://example.invalid/service.git",
    ]);
    repo.run_git(&["update-ref", "refs/remotes/team/service/main", &head]);
    repo.run_git(&["tag", "lightweight"]);
    repo.run_git(&["tag", "-a", "annotated", "-m", "release notes"]);
    write(
        repo.path().join(".gitmodules"),
        "[submodule \"vendor\"]\n\tpath = vendor/lib\n\turl = https://example.invalid/lib.git\n",
    )
    .unwrap();
    repo.run_git(&["add", ".gitmodules"]);
    repo.run_git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{head},vendor/lib"),
    ]);
    repo.run_git(&["commit", "-m", "add submodule"]);
    let linked = repo.path().join("linked tree");
    repo.run_git(&[
        "worktree",
        "add",
        "--detach",
        linked.to_str().unwrap(),
        "HEAD",
    ]);
    repo.run_git(&[
        "worktree",
        "lock",
        "--reason",
        "test lock",
        linked.to_str().unwrap(),
    ]);
    let status = repo.run_git_capture(&["status", "--porcelain"]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let remotes = backend.repository_tab(T::Remotes).unwrap();
    assert!(remotes.iter().any(|item| matches!(item, I::Remote { name, fetch_url, push_url } if name == "origin" && fetch_url.starts_with("https:") && push_url.starts_with("ssh:"))));
    assert!(remotes.iter().any(|item| matches!(item, I::RemoteBranch { name, target } if name == "origin/main" && target == &head)));
    assert!(
        !remotes
            .iter()
            .any(|item| matches!(item, I::RemoteBranch { name, .. } if name == "origin/HEAD"))
    );
    assert_eq!(
        remotes
            .iter()
            .filter(
                |item| matches!(item, I::RemoteBranch { name, .. } if name == "team/service/main")
            )
            .count(),
        1
    );
    let service_index = remotes
        .iter()
        .position(|item| matches!(item, I::Remote { name, .. } if name == "team/service"))
        .unwrap();
    assert!(
        matches!(&remotes[service_index + 1], I::RemoteBranch { name, .. } if name == "team/service/main")
    );
    let tags = backend.repository_tab(T::Tags).unwrap();
    assert!(tags.iter().any(|item| matches!(item, I::Tag { name, target, summary } if name == "annotated" && target == &head && summary.contains("release notes"))));
    assert!(tags.iter().any(|item| matches!(item, I::Tag { name, target, .. } if name == "lightweight" && target == &head)));
    let trees = backend.repository_tab(T::Worktrees).unwrap();
    assert_eq!(trees.len(), 2);
    assert!(matches!(trees[0], I::Worktree { main: true, .. }));
    assert!(trees.iter().any(|item| matches!(item, I::Worktree { path, detached: true, locked: Some(reason), main: false, .. } if path.ends_with("linked tree") && reason == "test lock")));
    let modules = backend.repository_tab(T::Submodules).unwrap();
    assert!(modules.iter().any(|item| matches!(item, I::Submodule { path, url, status, .. } if path == "vendor/lib" && url.contains("example.invalid") && status == "uninitialized")));
    let reflog = backend.repository_tab(T::Reflog).unwrap();
    assert!(
        matches!(&reflog[0], I::Reflog { selector, target, message } if selector == "HEAD@{0}" && target.len() == 40 && message.contains("add submodule"))
    );
    assert_eq!(status, repo.run_git_capture(&["status", "--porcelain"]));
    // Read through a linked checkout too; the first record still identifies main.
    let mut linked_backend = HybridGitBackend::open(&linked).unwrap();
    assert_eq!(linked_backend.repository_tab(T::Worktrees).unwrap(), trees);
    repo.run_git(&["worktree", "unlock", linked.to_str().unwrap()]);
    repo.run_git(&["worktree", "remove", linked.to_str().unwrap()]);
}

#[test]
fn repository_tabs_handle_unborn_and_empty_repositories() {
    let repo = TmpGitRepo::new("empty-repository-tabs");
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    for tab in [T::Tags, T::Remotes, T::Submodules, T::Reflog] {
        assert!(backend.repository_tab(tab).unwrap().is_empty(), "{tab:?}");
    }
    assert_eq!(backend.repository_tab(T::Worktrees).unwrap().len(), 1);
}
