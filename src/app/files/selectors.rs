use super::FilesRenderView;
use crate::file_tree::FileTreeNodeStatus;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct SelectionTarget {
    path: PathBuf,
    all_staged: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StageTarget {
    Stage(PathBuf),
    Unstage(PathBuf),
}

pub struct FileSelection<'a> {
    files: FilesRenderView<'a>,
    active: bool,
}

impl<'a> FileSelection<'a> {
    pub fn is_visual_mode(&self) -> bool {
        self.active && self.files.visual_mode
    }
    pub fn new(files: FilesRenderView<'a>, active: bool) -> Self {
        Self { files, active }
    }

    pub fn selected_tree_node(&self) -> Option<&'a crate::file_tree::FileTreeNode> {
        if !self.active {
            return None;
        }
        self.files.tree_nodes.get(self.files.panel.selected()?)
    }

    pub fn toggle_stage_target(&self) -> Option<StageTarget> {
        let node = self.selected_tree_node()?;
        let staged = if node.is_dir {
            let index = self.files.panel.selected()?;
            let end = self.subtree_end_index(index);
            let selected = (index..=end).collect();
            self.selected_files_are_all_staged(&selected, &node.path)
        } else {
            matches!(node.status, FileTreeNodeStatus::Staged(_))
        };
        Some(if staged {
            StageTarget::Unstage(node.path.clone())
        } else {
            StageTarget::Stage(node.path.clone())
        })
    }

    pub fn visual_selected_indices(&self) -> HashSet<usize> {
        let mut set = HashSet::new();
        if !self.active || !self.files.visual_mode {
            return set;
        }
        let Some(current) = self.files.panel.selected() else {
            return set;
        };
        let anchor = self.files.visual_anchor.unwrap_or(current);
        let (start, end) = if anchor <= current {
            (anchor, current)
        } else {
            (current, anchor)
        };
        for idx in start..=end {
            set.insert(idx);
        }
        set
    }

    pub fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf> {
        if !self.active {
            return Vec::new();
        }

        if self.files.visual_mode {
            let selected = self.visual_selected_indices();
            return self.collect_commit_targets(&selected);
        }

        let Some(node) = self.selected_tree_node() else {
            return Vec::new();
        };
        vec![node.path.clone()]
    }

    pub fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf> {
        if !self.active {
            return Vec::new();
        }

        if self.files.visual_mode {
            let selected = self.visual_selected_indices();
            return self.collect_discard_targets(&selected);
        }

        let Some(index) = self.files.panel.selected() else {
            return Vec::new();
        };
        self.collect_discard_targets_for_index(index)
    }

    pub fn partition_toggle_targets(
        &self,
        selected: &HashSet<usize>,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let mut stage_paths = Vec::new();
        let mut unstage_paths = Vec::new();

        for target in self.collect_selection_targets(selected) {
            if target.all_staged {
                unstage_paths.push(target.path);
            } else {
                stage_paths.push(target.path);
            }
        }

        (stage_paths, unstage_paths)
    }

    pub fn collect_commit_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        self.collect_selection_targets(selected)
            .into_iter()
            .map(|t| t.path)
            .collect()
    }

    fn collect_discard_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        let mut targets = Vec::new();
        let mut covered = HashSet::new();
        let mut ordered: Vec<usize> = selected.iter().copied().collect();
        ordered.sort_unstable();

        for idx in ordered {
            if covered.contains(&idx) {
                continue;
            }
            let Some(node) = self.files.tree_nodes.get(idx) else {
                continue;
            };

            if node.is_dir {
                let end = self.subtree_end_index(idx);
                let fully_covered = (idx..=end).all(|i| selected.contains(&i));
                if fully_covered {
                    targets.extend(self.collect_discard_targets_in_range(idx, end));
                    for i in idx..=end {
                        covered.insert(i);
                    }
                }
                continue;
            }

            if is_discardable_status(&node.status) {
                targets.push(node.path.clone());
            }
            covered.insert(idx);
        }

        dedup_paths(targets)
    }

    fn collect_discard_targets_for_index(&self, index: usize) -> Vec<PathBuf> {
        let Some(node) = self.files.tree_nodes.get(index) else {
            return Vec::new();
        };
        if node.is_dir {
            let end = self.subtree_end_index(index);
            return self.collect_discard_targets_in_range(index, end);
        }
        if is_discardable_status(&node.status) {
            return vec![node.path.clone()];
        }
        Vec::new()
    }

    fn collect_discard_targets_in_range(&self, start: usize, end: usize) -> Vec<PathBuf> {
        let mut targets = Vec::new();
        for i in start..=end {
            let Some(node) = self.files.tree_nodes.get(i) else {
                continue;
            };
            if node.is_dir {
                continue;
            }
            if is_discardable_status(&node.status) {
                targets.push(node.path.clone());
            }
        }
        dedup_paths(targets)
    }

    fn collect_selection_targets(&self, selected: &HashSet<usize>) -> Vec<SelectionTarget> {
        let mut targets = Vec::new();
        let mut covered = HashSet::new();
        let mut ordered: Vec<usize> = selected.iter().copied().collect();
        ordered.sort_unstable();

        for idx in ordered {
            if covered.contains(&idx) {
                continue;
            }
            let Some(node) = self.files.tree_nodes.get(idx) else {
                continue;
            };

            if node.is_dir {
                let end = self.subtree_end_index(idx);
                let fully_covered = (idx..=end).all(|i| selected.contains(&i));
                if fully_covered {
                    let all_staged = self.selected_files_are_all_staged(selected, &node.path);
                    targets.push(SelectionTarget {
                        path: node.path.clone(),
                        all_staged,
                    });
                    for i in idx..=end {
                        covered.insert(i);
                    }
                }
                continue;
            }

            let all_staged = matches!(node.status, FileTreeNodeStatus::Staged(_));
            targets.push(SelectionTarget {
                path: node.path.clone(),
                all_staged,
            });
            covered.insert(idx);
        }

        dedup_targets(targets)
    }

    fn selected_files_are_all_staged(&self, selected: &HashSet<usize>, dir_path: &Path) -> bool {
        let mut has_file = false;
        for idx in selected {
            let Some(node) = self.files.tree_nodes.get(*idx) else {
                continue;
            };
            if node.is_dir || !node.path.starts_with(dir_path) {
                continue;
            }
            has_file = true;
            if !matches!(node.status, FileTreeNodeStatus::Staged(_)) {
                return false;
            }
        }
        has_file
    }

    fn subtree_end_index(&self, index: usize) -> usize {
        let Some(node) = self.files.tree_nodes.get(index) else {
            return index;
        };
        if !node.is_dir {
            return index;
        }

        let base_depth = node.depth;
        let mut end = index;
        for i in index + 1..self.files.tree_nodes.len() {
            let n = &self.files.tree_nodes[i];
            if n.depth <= base_depth {
                break;
            }
            end = i;
        }
        end
    }
}

fn dedup_targets(mut targets: Vec<SelectionTarget>) -> Vec<SelectionTarget> {
    let mut seen = HashSet::<PathBuf>::new();
    targets.retain(|t| seen.insert(t.path.clone()));
    targets
}

fn dedup_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::<PathBuf>::new();
    paths.retain(|p| seen.insert(p.clone()));
    paths
}

fn is_discardable_status(status: &FileTreeNodeStatus) -> bool {
    matches!(
        status,
        FileTreeNodeStatus::Staged(_) | FileTreeNodeStatus::Unstaged(_)
    )
}
