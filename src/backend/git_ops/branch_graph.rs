use std::process::Command;

use anyhow::{anyhow, Context, Result};

use super::repo::GitRepo;

pub fn get_branch_graph(repo: &GitRepo, branch_name: &str, limit: usize) -> Result<String> {
    let workdir = repo
        .repo
        .workdir()
        .context("Repository has no working directory")?;
    let revision = format!("refs/heads/{branch_name}");

    let output = Command::new("git")
        .arg("log")
        .arg("--graph")
        .arg("--decorate")
        .arg("--color=always")
        .arg(format!("--max-count={limit}"))
        .arg(revision)
        .current_dir(workdir)
        .output()
        .context("failed to execute git log --graph")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow!("git log --graph failed: {stderr}"));
    }

    let graph = String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string();
    if graph.is_empty() {
        Ok("(no commits found for this branch)".to_string())
    } else {
        Ok(graph)
    }
}
