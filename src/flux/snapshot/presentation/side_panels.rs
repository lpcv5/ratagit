//! Resolve side-panel view routing and domain metadata outside the renderers.
use super::AppStateSnapshot;
use crate::app::revisions::CommitsRenderView;
use crate::app::SidePanel;
use crate::git::CommitSyncState;
use crate::ui::components::atoms::Tone;
use crate::ui::components::organisms::props::*;

#[cfg(test)]
mod tests;

fn title_with_search(title: &str, summary: Option<&str>) -> String {
    match summary {
        Some(summary) => format!("{title} [{summary}]"),
        None => title.to_owned(),
    }
}

fn commits_content<'a>(
    view: CommitsRenderView<'a>,
) -> RevisionContent<'a, Vec<CommitRowProps<'a>>> {
    if view.tree_mode.active {
        return RevisionContent::Tree {
            nodes: view.tree_mode.nodes,
            view_id: view.tree_mode.selected_source.cloned(),
        };
    }
    let has_highlight = !view.highlighted_oids.is_empty();
    RevisionContent::List(
        view.items
            .iter()
            .map(|commit| CommitRowProps {
                oid: &commit.oid,
                author: &commit.author,
                message: &commit.message,
                tone: match commit.sync_state {
                    CommitSyncState::DefaultBranch => Tone::Success,
                    CommitSyncState::RemoteBranch => Tone::Warning,
                    CommitSyncState::LocalOnly => Tone::Danger,
                },
                graph: commit
                    .graph
                    .iter()
                    .map(|cell| {
                        let highlighted = if !cell.pipe_oids.is_empty() {
                            cell.pipe_oids
                                .iter()
                                .any(|oid| view.highlighted_oids.contains(oid))
                        } else {
                            cell.pipe_oid
                                .as_ref()
                                .is_some_and(|oid| view.highlighted_oids.contains(oid))
                        };
                        GraphCellProps {
                            text: &cell.text,
                            dimmed: has_highlight && !highlighted,
                        }
                    })
                    .collect(),
            })
            .collect(),
    )
}

impl AppStateSnapshot<'_> {
    fn panel_active(&self, panel: SidePanel) -> bool {
        !self.context.diff_focused && self.active_panel == panel
    }

    pub fn files_panel_props(&self) -> FilesPanelProps<'_> {
        FilesPanelProps {
            header: PanelHeader {
                title: title_with_search(
                    if self.files.visual_mode {
                        "Files [VISUAL]"
                    } else {
                        "Files"
                    },
                    self.render_cache.files_search_summary.as_deref(),
                ),
                is_active: self.panel_active(SidePanel::Files),
                selected: self.files.panel.selected(),
                search_query: self.files_search_query,
            },
            nodes: self.files.tree_nodes,
            selected_indices: &self.render_cache.files_visual_selected_indices,
        }
    }

    pub fn branches_panel_props(&self) -> BranchesPanelProps<'_> {
        let view = self.branches;
        if view.commits_subview_active {
            let title = match view.commits_subview_source {
                Some(name) if view.commits_subview_loading => {
                    format!("Branch Commits: {name} [Loading...] [Esc Back]")
                }
                Some(name) => format!("Branch Commits: {name} [Esc Back]"),
                None => "Branch Commits [Esc Back]".into(),
            };
            return BranchesPanelProps::Commits {
                props: CommitsPanelProps {
                    header: PanelHeader {
                        title: title_with_search(
                            &title,
                            self.render_cache.branches_search_summary.as_deref(),
                        ),
                        is_active: self.panel_active(SidePanel::LocalBranches),
                        selected: view.commits_subview.panel.selected(),
                        search_query: self.branches_search_query,
                    },
                    content: commits_content(view.commits_subview),
                },
                view_id: view.commits_subview_source.map(String::as_str),
            };
        }
        BranchesPanelProps::List {
            header: PanelHeader {
                title: title_with_search(
                    "Local Branches",
                    self.render_cache.branches_search_summary.as_deref(),
                ),
                is_active: self.panel_active(SidePanel::LocalBranches),
                selected: view.panel.selected(),
                search_query: self.branches_search_query,
            },
            rows: view
                .items
                .iter()
                .map(|branch| BranchRowProps {
                    name: &branch.name,
                    is_current: branch.is_current,
                })
                .collect(),
        }
    }

    pub fn commits_panel_props(&self) -> CommitsPanelProps<'_> {
        let view = self.commits;
        let title = if view.tree_mode.active {
            match view.tree_mode.selected_source {
                Some(oid) => format!("Commit Files {} [Esc Back]", &oid[..oid.len().min(7)]),
                None => "Commit Files [Esc Back]".into(),
            }
        } else {
            "Commits".into()
        };
        CommitsPanelProps {
            header: PanelHeader {
                title: title_with_search(
                    &title,
                    self.render_cache.commits_search_summary.as_deref(),
                ),
                is_active: self.panel_active(SidePanel::Commits),
                selected: view.panel.selected(),
                search_query: self.commits_search_query,
            },
            content: commits_content(view),
        }
    }

    pub fn stash_panel_props(&self) -> StashPanelProps<'_> {
        let view = self.stash;
        let title = if view.tree_mode.active {
            match view.tree_mode.selected_source {
                Some(index) => format!("Stash Files stash@{{{index}}} [Esc Back]"),
                None => "Stash Files [Esc Back]".into(),
            }
        } else {
            "Stash".into()
        };
        StashPanelProps {
            header: PanelHeader {
                title: title_with_search(&title, self.render_cache.stash_search_summary.as_deref()),
                is_active: self.panel_active(SidePanel::Stash),
                selected: view.panel.selected(),
                search_query: self.stash_search_query,
            },
            content: if view.tree_mode.active {
                RevisionContent::Tree {
                    nodes: view.tree_mode.nodes,
                    view_id: view.tree_mode.selected_source.map(usize::to_string),
                }
            } else {
                RevisionContent::List(
                    view.items
                        .iter()
                        .map(|stash| StashRowProps {
                            index: stash.index,
                            message: &stash.message,
                        })
                        .collect(),
                )
            },
        }
    }

    pub fn side_panel_layout_props(&self) -> SidePanelLayoutProps {
        SidePanelLayoutProps {
            stash_focused: self.active_panel == SidePanel::Stash,
            focus_index: match self.active_panel {
                SidePanel::Files => Some(0),
                SidePanel::LocalBranches => Some(1),
                SidePanel::Commits => Some(2),
                SidePanel::Stash => None,
            },
            focus_item_count: match self.active_panel {
                SidePanel::Files => self.files.tree_nodes.len(),
                SidePanel::LocalBranches if self.branches.commits_subview_active => {
                    self.branches.commits_subview.items.len()
                }
                SidePanel::LocalBranches => self.branches.items.len(),
                // Preserve panel sizing when switching between a commit list and its tree.
                SidePanel::Commits => self.commits.items.len(),
                SidePanel::Stash => 0,
            },
        }
    }
}
