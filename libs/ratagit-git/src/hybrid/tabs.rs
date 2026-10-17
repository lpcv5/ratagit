use super::HybridGitBackend;
use crate::GitError;
use git2::{BranchType, ErrorCode};
use ratagit_core::{RepositoryItem as Item, RepositoryTab as Tab, tabs::REPOSITORY_LIST_LIMIT};

impl HybridGitBackend {
    pub(super) fn read_repository_tab(&mut self, tab: Tab) -> Result<Vec<Item>, GitError> {
        match tab {
            Tab::Worktrees => self.cli.tab_worktrees(),
            Tab::Remotes => {
                let mut items = Vec::new();
                let remotes = self.repo.remotes()?;
                let mut branches = self.repo.branches(Some(BranchType::Remote))?;
                // Walk refs once, and retain a bounded window before grouping.
                let branches = branches
                    .by_ref()
                    .filter_map(|entry| match entry {
                        Ok((branch, _)) if branch.get().symbolic_target().is_some() => None,
                        other => Some(other),
                    })
                    .take(REPOSITORY_LIST_LIMIT)
                    .map(|entry| {
                        let (branch, _) = entry?;
                        Ok((
                            branch.name()?.unwrap_or("").to_owned(),
                            branch
                                .get()
                                .target()
                                .map(|oid| oid.to_string())
                                .unwrap_or_default(),
                        ))
                    })
                    .collect::<Result<Vec<_>, git2::Error>>()?;
                for name in remotes.iter().flatten().take(REPOSITORY_LIST_LIMIT) {
                    if items.len() >= REPOSITORY_LIST_LIMIT {
                        break;
                    }
                    let remote = self.repo.find_remote(name)?;
                    items.push(Item::Remote {
                        name: name.to_owned(),
                        fetch_url: remote.url().unwrap_or("").to_owned(),
                        push_url: remote.pushurl().or(remote.url()).unwrap_or("").to_owned(),
                    });
                    for (branch, target) in &branches {
                        if items.len() >= REPOSITORY_LIST_LIMIT {
                            break;
                        }
                        let owner = remotes
                            .iter()
                            .flatten()
                            .filter(|remote| {
                                branch
                                    .strip_prefix(remote)
                                    .is_some_and(|suffix| suffix.starts_with('/'))
                            })
                            .max_by_key(|remote| remote.len());
                        if owner == Some(name) {
                            items.push(Item::RemoteBranch {
                                name: branch.clone(),
                                target: target.clone(),
                            });
                        }
                    }
                }
                Ok(items)
            }
            Tab::Tags => {
                let mut items = Vec::new();
                for reference in self
                    .repo
                    .references_glob("refs/tags/*")?
                    .take(REPOSITORY_LIST_LIMIT)
                {
                    let reference = reference?;
                    let resolved = reference.resolve()?;
                    let object = self.repo.find_object(
                        resolved
                            .target()
                            .ok_or_else(|| GitError::new("tag has no direct target"))?,
                        None,
                    )?;
                    let summary = object
                        .as_tag()
                        .and_then(|tag| tag.message())
                        .or_else(|| object.as_commit().and_then(|commit| commit.summary()))
                        .unwrap_or("")
                        .chars()
                        .take(4096)
                        .collect();
                    let target = reference
                        .peel_to_commit()
                        .map(|commit| commit.id())
                        .unwrap_or(object.id())
                        .to_string();
                    items.push(Item::Tag {
                        name: reference.shorthand().unwrap_or("").to_owned(),
                        target,
                        summary,
                    });
                }
                Ok(items)
            }
            Tab::Reflog => {
                let reflog = match self.repo.reflog("HEAD") {
                    Ok(log) => log,
                    Err(error) if error.code() == ErrorCode::NotFound => return Ok(Vec::new()),
                    Err(error) => return Err(error.into()),
                };
                Ok(reflog
                    .iter()
                    .take(REPOSITORY_LIST_LIMIT)
                    .enumerate()
                    .map(|(index, entry)| Item::Reflog {
                        selector: format!("HEAD@{{{index}}}"),
                        target: entry.id_new().to_string(),
                        message: entry.message().unwrap_or("").chars().take(4096).collect(),
                    })
                    .collect())
            }
            Tab::Submodules => {
                let mut items = Vec::new();
                for submodule in self
                    .repo
                    .submodules()?
                    .into_iter()
                    .take(REPOSITORY_LIST_LIMIT)
                {
                    let name = submodule.name().unwrap_or("");
                    let status = self
                        .repo
                        .submodule_status(name, git2::SubmoduleIgnore::Dirty)?;
                    let status = if !status.contains(git2::SubmoduleStatus::IN_WD) {
                        "uninitialized"
                    } else if status.contains(git2::SubmoduleStatus::WD_MODIFIED) {
                        "HEAD differs from index"
                    } else {
                        "initialized"
                    };
                    items.push(Item::Submodule {
                        path: submodule.path().to_string_lossy().into_owned(),
                        url: submodule.url().unwrap_or("").to_owned(),
                        head: submodule
                            .workdir_id()
                            .or(submodule.index_id())
                            .map(|id| id.to_string())
                            .unwrap_or_default(),
                        status: status.to_owned(),
                    });
                }
                Ok(items)
            }
        }
    }
}
