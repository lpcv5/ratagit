use anyhow::{Context, Result};
use git2::BranchType;
use std::collections::HashSet;
use std::process::Command;

use super::repo::GitRepo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // Reserved for rebasing/cherry-pick/conflict states parity
pub enum CommitStatus {
    #[default]
    None,
    Unpushed,
    Pushed,
    Merged,
    Rebasing,
    CherryPickingOrReverting,
    Conflicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitDivergence {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Debug, Clone, Default)]
pub struct CommitEntry {
    pub short_id: String,
    pub id: String,
    pub summary: String,
    pub body: Option<String>,
    #[allow(dead_code)] // Used in tests
    pub author: String,
    #[allow(dead_code)] // Used by commit panel author column
    pub author_name: String,
    #[allow(dead_code)] // Reserved for future use
    pub author_email: String,
    #[allow(dead_code)] // Reserved for future use
    pub timestamp: i64,
    #[allow(dead_code)] // Reserved for future bisect/todo support
    pub parents: Vec<String>,
    #[allow(dead_code)] // Reserved for future divergence view
    pub divergence: CommitDivergence,
    #[allow(dead_code)] // Decorations from git log (%D)
    pub decorations: String,
    #[allow(dead_code)] // Parsed tags from decorations
    pub tags: Vec<String>,
    #[allow(dead_code)] // Commit status used for hash coloring
    pub status: CommitStatus,
    pub graph_cells: Vec<crate::backend::git_ops::GraphCell>,
    #[allow(dead_code)] // Local branch head marker candidate
    pub is_branch_head: bool,
}

pub fn get_commits(repo: &GitRepo, limit: usize) -> Result<Vec<CommitEntry>> {
    load_commits(repo, "HEAD", limit, current_branch_name(repo)?)
}

pub fn get_commits_for_branch(
    repo: &GitRepo,
    branch_name: &str,
    limit: usize,
) -> Result<Vec<CommitEntry>> {
    let ref_spec = format!("refs/heads/{branch_name}");
    load_commits(repo, &ref_spec, limit, Some(branch_name.to_string()))
}

fn load_commits(
    repo: &GitRepo,
    ref_spec: &str,
    limit: usize,
    current_branch_name: Option<String>,
) -> Result<Vec<CommitEntry>> {
    let workdir = repo
        .repo
        .workdir()
        .context("Repository has no working directory")?;
    let log_output = git_log_output(workdir, ref_spec, limit)?;
    let main_branches = collect_main_branches(repo);
    let branch_heads = collect_visualized_branch_heads(repo, current_branch_name, &main_branches);
    let unmerged_hashes = if main_branches.is_empty() {
        None
    } else {
        Some(git_rev_list_set(workdir, ref_spec, &main_branches))
    };
    let unpushed_hashes = upstream_ref(repo, ref_spec)
        .map(|upstream| {
            let mut excludes = Vec::with_capacity(main_branches.len() + 1);
            excludes.push(upstream);
            excludes.extend(main_branches.iter().cloned());
            git_rev_list_set(workdir, ref_spec, &excludes)
        })
        .filter(|set| !set.is_empty());

    let mut commits = parse_commit_records(&log_output)?;
    let graph = super::commit_graph::render_commit_graph(&commits);
    for (commit, cells) in commits.iter_mut().zip(graph.into_iter()) {
        commit.graph_cells = cells;
        commit.is_branch_head = branch_heads.contains(&commit.id);
        commit.status = classify_commit_status(
            &commit.id,
            unmerged_hashes.as_ref(),
            unpushed_hashes.as_ref(),
        );
    }

    Ok(commits)
}

fn short_oid(oid: &str) -> String {
    oid.chars().take(8).collect()
}

fn current_branch_name(repo: &GitRepo) -> Result<Option<String>> {
    let head = repo.repo.head()?;
    if !head.is_branch() {
        return Ok(None);
    }
    Ok(head.shorthand().map(str::to_string))
}

fn collect_main_branches(repo: &GitRepo) -> Vec<String> {
    ["main", "master"]
        .into_iter()
        .filter(|branch| {
            let full_ref = format!("refs/heads/{branch}");
            repo.repo.find_reference(&full_ref).is_ok()
        })
        .map(|branch| format!("refs/heads/{branch}"))
        .collect()
}

fn collect_visualized_branch_heads(
    repo: &GitRepo,
    current_branch_name: Option<String>,
    main_branches: &[String],
) -> HashSet<String> {
    let current = current_branch_name.unwrap_or_default();
    let main_names: HashSet<String> = main_branches
        .iter()
        .filter_map(|full_ref| full_ref.rsplit('/').next().map(str::to_string))
        .collect();

    let mut heads = HashSet::new();
    if let Ok(branches) = repo.repo.branches(Some(BranchType::Local)) {
        for branch_result in branches {
            let Ok((branch, _)) = branch_result else {
                continue;
            };
            let Ok(Some(name)) = branch.name() else {
                continue;
            };
            if name == current || main_names.contains(name) {
                continue;
            }
            if let Some(target) = branch.get().target() {
                heads.insert(target.to_string());
            }
        }
    }
    heads
}

fn upstream_ref(repo: &GitRepo, ref_spec: &str) -> Option<String> {
    let branch_name = if ref_spec == "HEAD" {
        current_branch_name(repo).ok().flatten()?
    } else {
        ref_spec.strip_prefix("refs/heads/")?.to_string()
    };
    let branch = repo
        .repo
        .find_branch(&branch_name, BranchType::Local)
        .ok()?;
    let upstream = branch.upstream().ok()?;
    upstream.name().ok().flatten().map(str::to_string)
}

fn git_log_output(workdir: &std::path::Path, ref_spec: &str, limit: usize) -> Result<String> {
    // 0x1f (US) between fields, 0x1e (RS) between records.
    let format = "%x1f%H%x1f%at%x1f%aN%x1f%ae%x1f%P%x1f%m%x1f%D%x1f%s%x1f%b%x1e";

    let output = Command::new("git")
        .current_dir(workdir)
        .arg("log")
        .arg(ref_spec)
        .arg("--date-order")
        .arg(format!("--max-count={limit}"))
        .arg("--abbrev=40")
        .arg("--no-show-signature")
        .arg(format!("--pretty=format:{format}"))
        .output()
        .context("failed to execute git log for commits panel")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!("git log failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn git_rev_list_set(
    workdir: &std::path::Path,
    include_ref: &str,
    exclude_refs: &[String],
) -> HashSet<String> {
    let mut command = Command::new("git");
    command
        .current_dir(workdir)
        .arg("rev-list")
        .arg(include_ref);
    for exclude_ref in exclude_refs {
        command.arg(format!("^{exclude_ref}"));
    }

    let Ok(output) = command.output() else {
        return HashSet::new();
    };
    if !output.status.success() {
        return HashSet::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_commit_records(raw: &str) -> Result<Vec<CommitEntry>> {
    let mut commits = Vec::new();
    for record in raw.split('\u{1e}') {
        if record.trim().is_empty() {
            continue;
        }

        let trimmed = record.trim_start_matches('\n').trim_start_matches('\u{1f}');
        let fields: Vec<&str> = trimmed.split('\u{1f}').collect();
        if fields.len() < 9 {
            anyhow::bail!("Malformed git log record for commits panel");
        }

        let id = fields[0].to_string();
        let timestamp = fields[1].parse::<i64>().unwrap_or_default();
        let author_name = fields[2].to_string();
        let author_email = fields[3].to_string();
        let parents = parse_parents(fields[4]);
        let divergence = parse_divergence(fields[5]);
        let decorations = fields[6].trim().to_string();
        let summary = if fields[7].trim().is_empty() {
            "(no summary)".to_string()
        } else {
            fields[7].to_string()
        };
        let body = fields[8].trim();
        let tags = parse_tags(&decorations);

        commits.push(CommitEntry {
            short_id: short_oid(&id),
            id,
            summary,
            body: (!body.is_empty()).then(|| body.to_string()),
            author: format!("{author_name} <{author_email}>"),
            author_name,
            author_email,
            timestamp,
            parents,
            divergence,
            decorations,
            tags,
            status: CommitStatus::None,
            graph_cells: vec![],
            is_branch_head: false,
        });
    }

    Ok(commits)
}

fn parse_parents(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(str::to_string).collect()
}

fn parse_divergence(raw: &str) -> CommitDivergence {
    match raw {
        "<" => CommitDivergence::Left,
        ">" => CommitDivergence::Right,
        _ => CommitDivergence::None,
    }
}

fn parse_tags(decorations: &str) -> Vec<String> {
    decorations
        .split(',')
        .filter_map(|part| part.trim().strip_prefix("tag: ").map(str::to_string))
        .collect()
}

fn classify_commit_status(
    commit_hash: &str,
    unmerged_hashes: Option<&HashSet<String>>,
    unpushed_hashes: Option<&HashSet<String>>,
) -> CommitStatus {
    let is_unmerged = unmerged_hashes.is_none_or(|set| set.contains(commit_hash));
    if !is_unmerged {
        return CommitStatus::Merged;
    }
    if unpushed_hashes.is_some_and(|set| set.contains(commit_hash)) {
        return CommitStatus::Unpushed;
    }
    CommitStatus::Pushed
}

pub fn get_commit_message(repo: &GitRepo, commit_id: &str) -> Result<String> {
    let oid = git2::Oid::from_str(commit_id)?;
    let commit = repo.repo.find_commit(oid)?;
    Ok(commit.message().unwrap_or("").to_string())
}

/// Create a new commit with staged files
pub fn commit(repo: &GitRepo, message: &str) -> Result<()> {
    let mut index = repo.repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.repo.find_tree(tree_id)?;
    let sig = repo.repo.signature()?;

    // Get parent commit (HEAD)
    let parent_commit = match repo.repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(_) => None, // Initial commit
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    repo.repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(())
}

pub fn amend_commit(repo: &GitRepo, message: &str) -> Result<()> {
    let head = repo.repo.head()?;
    let commit = head.peel_to_commit()?;

    // Use commit.amend() to properly amend the commit
    commit.amend(
        Some("HEAD"),
        None, // author - None means keep original
        None, // committer - None means use current signature
        None, // message_encoding - None means UTF-8
        Some(message),
        None, // tree - None means keep original tree
    )?;

    Ok(())
}

pub fn amend_commit_with_files(
    repo: &GitRepo,
    commit_id: &str,
    message: &str,
    paths: &[String],
) -> Result<()> {
    // Stage the specified files
    let mut index = repo.repo.index()?;
    for path in paths {
        index.add_path(std::path::Path::new(path))?;
    }
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.repo.find_tree(tree_id)?;

    // Get the target commit
    let oid = git2::Oid::from_str(commit_id)?;
    let commit = repo.repo.find_commit(oid)?;

    // Check if this is HEAD
    let head = repo.repo.head()?;
    let head_commit = head.peel_to_commit()?;

    if commit.id() == head_commit.id() {
        // Amending HEAD - straightforward
        commit.amend(Some("HEAD"), None, None, None, Some(message), Some(&tree))?;
    } else {
        // Amending a non-HEAD commit: rewrite the target commit and rebase descendants
        amend_non_head_commit(repo, &commit, message, &tree, &head_commit)?;
    }

    Ok(())
}

/// Rewrite a non-HEAD commit with a new tree, then rebase all descendant commits on top.
fn amend_non_head_commit(
    repo: &GitRepo,
    target: &git2::Commit,
    message: &str,
    new_tree: &git2::Tree,
    head_commit: &git2::Commit,
) -> Result<()> {
    // Collect commits from target (exclusive) up to HEAD (inclusive), oldest first
    let mut descendants: Vec<git2::Commit> = Vec::new();
    let mut current = head_commit.clone();
    loop {
        if current.id() == target.id() {
            break;
        }
        descendants.push(current.clone());
        if current.parent_count() == 0 {
            anyhow::bail!("Target commit not found in ancestry of HEAD");
        }
        current = current.parent(0)?;
    }
    descendants.reverse(); // oldest descendant first

    // Create the amended target commit (no ref update yet)
    let amended_oid = target.amend(
        None, // don't update any ref
        None,
        None,
        None,
        Some(message),
        Some(new_tree),
    )?;
    let mut new_parent_oid = amended_oid;

    // Replay each descendant commit on top of the amended commit
    let sig = repo.repo.signature()?;
    for desc in &descendants {
        let new_parent = repo.repo.find_commit(new_parent_oid)?;
        let desc_tree = desc.tree()?;
        new_parent_oid = repo.repo.commit(
            None, // no ref update yet
            &desc.author(),
            &sig,
            desc.message().unwrap_or(""),
            &desc_tree,
            &[&new_parent],
        )?;
    }

    // Update HEAD to point to the new tip
    let new_head = repo.repo.find_object(new_parent_oid, None)?;
    repo.repo.reset(&new_head, git2::ResetType::Soft, None)?;

    Ok(())
}

pub fn checkout_commit(repo: &GitRepo, commit_id: &str) -> Result<()> {
    let oid = git2::Oid::from_str(commit_id)?;
    let commit = repo.repo.find_commit(oid)?;
    repo.repo.checkout_tree(commit.as_object(), None)?;
    repo.repo.set_head_detached(oid)?;
    Ok(())
}

pub fn cherry_pick_commits(repo: &GitRepo, commit_ids: &[String]) -> Result<()> {
    if commit_ids.is_empty() {
        return Ok(());
    }

    let workdir = repo
        .repo
        .workdir()
        .context("Repository has no working directory")?;

    let mut has_merge_commit = false;
    for commit_id in commit_ids {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = repo.repo.find_commit(oid)?;
        if commit.parent_count() > 1 {
            has_merge_commit = true;
            break;
        }
    }

    let mut cmd = Command::new("git");
    cmd.current_dir(workdir);
    cmd.arg("cherry-pick").arg("--allow-empty");

    if has_merge_commit {
        cmd.arg("-m1");
    }

    for commit_id in commit_ids.iter().rev() {
        cmd.arg(commit_id);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to cherry-pick commits: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

pub fn reset_hard(repo: &GitRepo, target: &str) -> Result<()> {
    let obj = repo.repo.revparse_single(target)?;
    repo.repo.reset(&obj, git2::ResetType::Hard, None)?;
    Ok(())
}

pub fn reset_mixed(repo: &GitRepo, target: &str) -> Result<()> {
    let obj = repo.repo.revparse_single(target)?;
    repo.repo.reset(&obj, git2::ResetType::Mixed, None)?;
    Ok(())
}

pub fn reset_soft(repo: &GitRepo, target: &str) -> Result<()> {
    let obj = repo.repo.revparse_single(target)?;
    repo.repo.reset(&obj, git2::ResetType::Soft, None)?;
    Ok(())
}

pub fn revert_commit(repo: &GitRepo, commit_id: &str) -> Result<()> {
    let oid = git2::Oid::from_str(commit_id)?;
    let commit = repo.repo.find_commit(oid)?;
    let mut revert_opts = git2::RevertOptions::new();
    repo.repo.revert(&commit, Some(&mut revert_opts))?;

    // Check for conflicts before committing
    let mut index = repo.repo.index()?;
    if index.has_conflicts() {
        repo.repo.cleanup_state()?;
        anyhow::bail!(
            "Revert of {} produced conflicts — resolve them manually",
            &commit_id[..commit_id.len().min(8)]
        );
    }

    // Build the revert commit
    let sig = repo.repo.signature()?;
    let message = repo
        .repo
        .message()
        .unwrap_or_else(|_| format!("Revert \"{}\"", commit.summary().unwrap_or(commit_id)));
    let tree_id = index.write_tree()?;
    let tree = repo.repo.find_tree(tree_id)?;
    let head_commit = repo.repo.head()?.peel_to_commit()?;
    repo.repo
        .commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&head_commit])?;

    repo.repo.cleanup_state()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_test_repo() -> (tempfile::TempDir, GitRepo) {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path();

        let repo = git2::Repository::init(path).expect("Failed to init repo");

        {
            let mut config = repo.config().expect("Failed to get config");
            config
                .set_str("user.name", "Test User")
                .expect("Failed to set user.name");
            config
                .set_str("user.email", "test@example.com")
                .expect("Failed to set user.email");
        }

        {
            let sig = repo.signature().expect("Failed to create signature");
            let tree_id = {
                let mut index = repo.index().expect("Failed to get index");
                index.write_tree().expect("Failed to write tree")
            };
            let tree = repo.find_tree(tree_id).expect("Failed to find tree");
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .expect("Failed to create initial commit");
        }

        (temp_dir, GitRepo { repo })
    }

    fn add_commit(repo: &GitRepo, temp_dir: &tempfile::TempDir, message: &str) {
        let filename = format!("file_{}.txt", message.replace(' ', "_"));
        fs::write(temp_dir.path().join(&filename), "content").expect("Failed to write file");

        let mut index = repo.repo.index().expect("Failed to get index");
        index
            .add_path(Path::new(&filename))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo.repo.head().unwrap().peel_to_commit().unwrap();

        repo.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .expect("Failed to commit");

        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_get_commits_single() {
        let (_temp_dir, repo) = create_test_repo();
        let commits = get_commits(&repo, 10).expect("Failed to get commits");

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "Initial commit");
        assert_eq!(commits[0].author, "Test User <test@example.com>");
        assert_eq!(commits[0].short_id.len(), 8);
    }

    #[test]
    fn test_get_commits_multiple() {
        let (temp_dir, repo) = create_test_repo();

        add_commit(&repo, &temp_dir, "Second commit");
        add_commit(&repo, &temp_dir, "Third commit");

        let commits = get_commits(&repo, 10).expect("Failed to get commits");

        assert_eq!(commits.len(), 3);
        // Verify all commits are present
        let summaries: Vec<&str> = commits.iter().map(|c| c.summary.as_str()).collect();
        assert!(summaries.contains(&"Initial commit"));
        assert!(summaries.contains(&"Second commit"));
        assert!(summaries.contains(&"Third commit"));
        // Most recent should be first (Third commit)
        assert_eq!(commits[0].summary, "Third commit");
    }

    #[test]
    fn test_get_commits_with_limit() {
        let (temp_dir, repo) = create_test_repo();

        add_commit(&repo, &temp_dir, "Second commit");
        add_commit(&repo, &temp_dir, "Third commit");
        add_commit(&repo, &temp_dir, "Fourth commit");

        let commits = get_commits(&repo, 2).expect("Failed to get commits");

        // Should only get the 2 most recent commits
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "Fourth commit");
    }

    #[test]
    fn test_get_commits_for_branch() {
        let (temp_dir, repo) = create_test_repo();

        add_commit(&repo, &temp_dir, "Main commit");

        // Create a branch and add commits to it
        let head = repo.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        repo.repo
            .branch("feature", &commit, false)
            .expect("Failed to create branch");
        repo.repo
            .set_head("refs/heads/feature")
            .expect("Failed to set HEAD");

        add_commit(&repo, &temp_dir, "Feature commit");

        let commits = get_commits_for_branch(&repo, "feature", 10).expect("Failed to get commits");

        assert_eq!(commits.len(), 3);
        // Most recent commit should be first
        assert_eq!(commits[0].summary, "Feature commit");
        // Verify all commits are present
        let summaries: Vec<&str> = commits.iter().map(|c| c.summary.as_str()).collect();
        assert!(summaries.contains(&"Feature commit"));
        assert!(summaries.contains(&"Main commit"));
        assert!(summaries.contains(&"Initial commit"));
    }

    #[test]
    fn test_checkout_commit_detaches_head() {
        let (temp_dir, repo) = create_test_repo();
        add_commit(&repo, &temp_dir, "Commit A");
        add_commit(&repo, &temp_dir, "Commit B");

        let commits = get_commits(&repo, 10).expect("Failed to get commits");
        let target = commits
            .iter()
            .find(|c| c.summary == "Commit A")
            .expect("Commit A not found")
            .id
            .clone();

        checkout_commit(&repo, &target).expect("Failed to checkout commit");

        assert!(repo
            .repo
            .head_detached()
            .expect("Failed to check detached HEAD"));
        let head_oid = repo
            .repo
            .head()
            .expect("Failed to get HEAD")
            .target()
            .expect("HEAD has no target");
        assert_eq!(head_oid.to_string(), target);
    }

    #[test]
    fn test_cherry_pick_commits_applies_commit() {
        let (temp_dir, repo) = create_test_repo();

        let base_branch = repo
            .repo
            .head()
            .expect("Failed to get HEAD")
            .shorthand()
            .expect("Failed to get branch name")
            .to_string();
        let base_ref = format!("refs/heads/{base_branch}");
        let initial_commit = repo
            .repo
            .head()
            .expect("Failed to get HEAD")
            .peel_to_commit()
            .expect("Failed to peel initial commit");

        repo.repo
            .branch("feature", &initial_commit, false)
            .expect("Failed to create feature branch");
        repo.repo
            .set_head("refs/heads/feature")
            .expect("Failed to switch to feature branch");
        repo.repo
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .expect("Failed to checkout feature branch");

        fs::write(temp_dir.path().join("feature.txt"), "feature change")
            .expect("Failed to write feature file");
        let mut index = repo.repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("feature.txt"))
            .expect("Failed to stage feature file");
        index.write().expect("Failed to write index");
        let sig = repo.repo.signature().expect("Failed to create signature");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.repo.find_tree(tree_id).expect("Failed to find tree");
        let parent = repo
            .repo
            .head()
            .expect("Failed to get HEAD")
            .peel_to_commit()
            .expect("Failed to peel feature parent");
        let feature_commit_oid = repo
            .repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Feature commit",
                &tree,
                &[&parent],
            )
            .expect("Failed to create feature commit");

        repo.repo
            .set_head(&base_ref)
            .expect("Failed to switch back to base branch");
        repo.repo
            .checkout_head(Some(
                git2::build::CheckoutBuilder::new()
                    .force()
                    .remove_untracked(true),
            ))
            .expect("Failed to checkout base branch");

        cherry_pick_commits(&repo, &[feature_commit_oid.to_string()])
            .expect("Failed to cherry-pick commit");

        let head_summary = repo
            .repo
            .head()
            .expect("Failed to get HEAD")
            .peel_to_commit()
            .expect("Failed to peel HEAD")
            .summary()
            .unwrap_or("")
            .to_string();
        assert_eq!(head_summary, "Feature commit");
    }

    #[test]
    fn test_amend_non_head_commit() {
        let (temp_dir, repo) = create_test_repo();

        // Create a history: Initial -> A -> B -> C (HEAD)
        add_commit(&repo, &temp_dir, "Commit A");
        add_commit(&repo, &temp_dir, "Commit B");
        add_commit(&repo, &temp_dir, "Commit C");

        let commits = get_commits(&repo, 10).expect("Failed to get commits");
        assert_eq!(commits.len(), 4);

        // Find commits by summary (order is not guaranteed by get_commits)
        let commit_b_id = commits
            .iter()
            .find(|c| c.summary == "Commit B")
            .expect("Commit B not found")
            .id
            .clone();
        let commit_a_id = commits
            .iter()
            .find(|c| c.summary == "Commit A")
            .expect("Commit A not found")
            .id
            .clone();
        let commit_c_id = commits
            .iter()
            .find(|c| c.summary == "Commit C")
            .expect("Commit C not found")
            .id
            .clone();

        // Create a new file to amend into commit B
        let new_file = "amended_file.txt";
        fs::write(temp_dir.path().join(new_file), "amended content").expect("Failed to write file");

        // Amend commit B with the new file
        amend_commit_with_files(
            &repo,
            &commit_b_id,
            "Commit B (amended)",
            &[new_file.to_string()],
        )
        .expect("Failed to amend commit");

        let new_commits = get_commits(&repo, 10).expect("Failed to get commits");
        assert_eq!(new_commits.len(), 4);

        let summaries: Vec<&str> = new_commits.iter().map(|c| c.summary.as_str()).collect();
        assert!(summaries.contains(&"Commit C"));
        assert!(summaries.contains(&"Commit B (amended)"));
        assert!(summaries.contains(&"Commit A"));
        assert!(summaries.contains(&"Initial commit"));
        assert!(!summaries.contains(&"Commit B")); // original B is gone

        // A is unchanged (it's before B in history)
        let new_a = new_commits
            .iter()
            .find(|c| c.summary == "Commit A")
            .unwrap();
        assert_eq!(new_a.id, commit_a_id);

        // B and C have new IDs (history rewritten)
        let new_b = new_commits
            .iter()
            .find(|c| c.summary == "Commit B (amended)")
            .unwrap();
        let new_c = new_commits
            .iter()
            .find(|c| c.summary == "Commit C")
            .unwrap();
        assert_ne!(new_b.id, commit_b_id);
        assert_ne!(new_c.id, commit_c_id);
    }
}
