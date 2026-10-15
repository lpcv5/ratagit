use super::FilesState;
use std::path::{Path, PathBuf};

pub enum FilesAction<'a> {
    ReplaceStatus(crate::shared::Shared<crate::git::GitStatus>),
    Next,
    Previous,
    SelectIndex(usize),
    ToggleDirectory,
    CollapseAll,
    ExpandAll,
    ToggleVisual,
    #[cfg(test)]
    ClearVisual,
    StagePath(&'a Path),
    UnstagePath(&'a Path),
    DiscardPaths(&'a [PathBuf]),
    StageFinished {
        path: &'a Path,
        result: &'a Result<(), crate::git::GitError>,
    },
    UnstageFinished {
        path: &'a Path,
        result: &'a Result<(), crate::git::GitError>,
    },
    DiscardPathsFinished {
        paths: &'a [PathBuf],
        result: &'a Result<(), crate::git::GitError>,
    },
    ToggleStageSelection,
    ToggleStageSelectionFinished(&'a Result<(usize, usize), crate::git::GitError>),
    DiscardSelection,
    DiscardSelectionFinished(&'a Result<(), crate::git::GitError>),
    PrepareCommitSelection,
    PrepareCommitFinished(&'a Result<usize, crate::git::GitError>),
}

/// Owned requests; this feature never accesses a repository or Flux.
#[derive(Debug, PartialEq, Eq)]
pub enum FileWrite {
    Stage(PathBuf),
    Unstage(PathBuf),
    Discard(Vec<PathBuf>),
    ToggleSelection {
        stage: Vec<PathBuf>,
        unstage: Vec<PathBuf>,
    },
    PrepareCommit(Vec<PathBuf>),
}

#[derive(Default)]
pub struct FilesUpdate {
    pub write: Option<FileWrite>,
    pub log: Option<(String, bool)>,
    pub refresh_status: bool,
    pub open_commit_editor: bool,
    pub reload_diff: bool,
    pub redraw: bool,
}

impl FilesState {
    pub fn update(&mut self, action: FilesAction<'_>, active: bool) -> FilesUpdate {
        let mut output = FilesUpdate::default();
        match action {
            FilesAction::ReplaceStatus(status) => self.replace_status(status),
            FilesAction::Next | FilesAction::Previous => {
                let count = self.panel.tree_nodes.len();
                if count != 0 {
                    let index = self
                        .panel
                        .panel
                        .selected()
                        .map(|index| {
                            if matches!(action, FilesAction::Next) {
                                (index + 1).min(count - 1)
                            } else {
                                index.saturating_sub(1)
                            }
                        })
                        .unwrap_or(0);
                    self.panel.panel.select(Some(index));
                }
            }
            FilesAction::SelectIndex(index) => self.panel.panel.select(Some(index)),
            FilesAction::ToggleDirectory => {
                let path = self
                    .selection(active)
                    .selected_tree_node()
                    .filter(|node| node.is_dir)
                    .map(|node| node.path.clone());
                if let Some(path) = path {
                    if !self.panel.expanded_dirs.remove(&path) {
                        self.panel.expanded_dirs.insert(path);
                    }
                }
                self.rebuild_tree();
            }
            FilesAction::CollapseAll => {
                self.panel.expanded_dirs.clear();
                self.rebuild_tree();
            }
            FilesAction::ExpandAll => self.expand_all(),
            #[cfg(test)]
            FilesAction::ClearVisual => self.clear_visual_selection(),
            FilesAction::ToggleVisual if active => {
                if self.panel.visual_mode {
                    self.clear_visual_selection();
                } else {
                    self.panel.visual_mode = true;
                    self.panel.visual_anchor = self.panel.panel.selected();
                }
                output.reload_diff = true;
                output.redraw = true;
            }
            FilesAction::StagePath(path) => {
                output.write = Some(FileWrite::Stage(path.to_path_buf()))
            }
            FilesAction::UnstagePath(path) => {
                output.write = Some(FileWrite::Unstage(path.to_path_buf()))
            }
            FilesAction::DiscardPaths(paths) => {
                if paths.is_empty() {
                    output.log = Some((
                        "discard blocked: no discardable selected items".into(),
                        false,
                    ));
                } else {
                    output.write = Some(FileWrite::Discard(paths.to_vec()));
                }
            }
            FilesAction::StageFinished { path, result }
            | FilesAction::UnstageFinished { path, result } => {
                let stage = matches!(action, FilesAction::StageFinished { .. });
                output.log = Some(match result {
                    Ok(()) => (
                        format!(
                            "{} {}",
                            if stage { "staged" } else { "unstaged" },
                            path.display()
                        ),
                        true,
                    ),
                    Err(error) => (
                        format!(
                            "{} failed {}: {}",
                            if stage { "stage" } else { "unstage" },
                            path.display(),
                            crate::app::git_error::message(error)
                        ),
                        false,
                    ),
                });
                output.redraw = result.is_ok();
            }
            FilesAction::DiscardPathsFinished { paths, result } => {
                output.log = Some(match result {
                    Ok(()) if paths.len() == 1 => {
                        (format!("discarded {}", paths[0].display()), true)
                    }
                    Ok(()) => (format!("discarded {} path(s)", paths.len()), true),
                    Err(error) => (
                        format!("discard failed: {}", crate::app::git_error::message(error)),
                        false,
                    ),
                });
                output.redraw = result.is_ok();
            }
            FilesAction::ToggleStageSelection if active => {
                let selection = self.selection(active);
                let (stage, unstage) =
                    selection.partition_toggle_targets(&selection.visual_selected_indices());
                output.write = Some(FileWrite::ToggleSelection { stage, unstage });
            }
            FilesAction::ToggleStageSelectionFinished(result) => {
                output.log = Some(match result {
                    Ok((staged, unstaged)) => (
                        format!("selection toggled: staged {staged}, unstaged {unstaged}"),
                        true,
                    ),
                    Err(error) => (
                        format!(
                            "selection toggle failed: {}",
                            crate::app::git_error::message(error)
                        ),
                        false,
                    ),
                });
                output.refresh_status = result.is_ok();
                output.redraw = result.is_ok();
            }
            FilesAction::DiscardSelection if active => {
                let paths = self
                    .selection(active)
                    .prepare_discard_targets_from_selection();
                if paths.is_empty() {
                    output.log = Some((
                        "discard blocked: no discardable selected items".into(),
                        false,
                    ));
                } else {
                    output.write = Some(FileWrite::Discard(paths));
                }
            }
            FilesAction::DiscardSelectionFinished(result) if result.is_ok() => {
                self.clear_visual_selection();
                output.redraw = true;
            }
            FilesAction::PrepareCommitSelection if active => {
                let selection = self.selection(active);
                output.write = Some(FileWrite::PrepareCommit(
                    selection.collect_commit_targets(&selection.visual_selected_indices()),
                ));
            }
            FilesAction::PrepareCommitFinished(result) => {
                match result {
                    Ok(0) => output.log = Some(("commit blocked: no selected items".into(), false)),
                    Ok(count) => {
                        self.clear_visual_selection();
                        output.refresh_status = true;
                        output.open_commit_editor = true;
                        output.log = Some((format!("commit: {count} selected target(s) staged; edit message/description"), true));
                        output.redraw = true;
                    }
                    Err(error) => {
                        output.log = Some((
                            format!(
                                "prepare commit failed: {}",
                                crate::app::git_error::message(error)
                            ),
                            false,
                        ))
                    }
                }
            }
            _ => {}
        }
        output
    }
}
