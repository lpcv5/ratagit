use super::{BranchesPanelState, CommitsPanelState, StashPanelState, TreeModeState};
use crate::app::PanelView;
use crate::file_tree::FileTreeNode;
use crate::git::{BranchInfo, CommitInfo, StashInfo};
use crate::shared::Shared;
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub struct RevisionTreeRenderView<'a, T> {
    pub active: bool,
    pub nodes: &'a [FileTreeNode],
    pub selected_source: T,
}

struct RevisionTreeRenderModel<T> {
    active: bool,
    nodes: Shared<Vec<FileTreeNode>>,
    selected_source: Option<T>,
}

impl<T: Clone> From<&TreeModeState<T>> for RevisionTreeRenderModel<T> {
    fn from(state: &TreeModeState<T>) -> Self {
        Self {
            active: state.active,
            nodes: state.nodes.clone(),
            selected_source: state.selected_source.clone(),
        }
    }
}

impl<T> RevisionTreeRenderModel<T> {
    fn view(&self) -> RevisionTreeRenderView<'_, Option<&T>> {
        RevisionTreeRenderView {
            active: self.active,
            nodes: &self.nodes,
            selected_source: self.selected_source.as_ref(),
        }
    }
}

impl<T> TreeModeState<T> {
    fn render_view(&self) -> RevisionTreeRenderView<'_, Option<&T>> {
        RevisionTreeRenderView {
            active: self.active,
            nodes: &self.nodes,
            selected_source: self.selected_source.as_ref(),
        }
    }
}

/// Feature projections contain only immutable data used by renderers/selectors.
#[derive(Clone, Copy)]
pub struct CommitsRenderView<'a> {
    pub panel: PanelView,
    pub items: &'a [CommitInfo],
    pub tree_mode: RevisionTreeRenderView<'a, Option<&'a String>>,
    pub highlighted_oids: &'a HashSet<String>,
}

pub struct CommitsRenderModel {
    panel: PanelView,
    items: Shared<Vec<CommitInfo>>,
    tree_mode: RevisionTreeRenderModel<String>,
    highlighted_oids: Shared<HashSet<String>>,
}

impl CommitsPanelState {
    pub fn render_view(&self) -> CommitsRenderView<'_> {
        CommitsRenderView {
            panel: self.panel.view(),
            items: &self.items,
            tree_mode: self.tree_mode.render_view(),
            highlighted_oids: &self.highlighted_oids,
        }
    }

    pub fn render_model(&self) -> CommitsRenderModel {
        CommitsRenderModel {
            panel: self.panel.view(),
            items: self.items.clone(),
            tree_mode: (&self.tree_mode).into(),
            highlighted_oids: self.highlighted_oids.clone(),
        }
    }
}

impl CommitsRenderModel {
    pub fn view(&self) -> CommitsRenderView<'_> {
        CommitsRenderView {
            panel: self.panel,
            items: &self.items,
            tree_mode: self.tree_mode.view(),
            highlighted_oids: &self.highlighted_oids,
        }
    }
}

#[derive(Clone, Copy)]
pub struct BranchesRenderView<'a> {
    pub panel: PanelView,
    pub items: &'a [BranchInfo],
    pub commits_subview_active: bool,
    pub commits_subview_loading: bool,
    pub commits_subview_source: Option<&'a String>,
    pub commits_subview: CommitsRenderView<'a>,
}

pub struct BranchesRenderModel {
    panel: PanelView,
    items: Shared<Vec<BranchInfo>>,
    commits_subview_active: bool,
    commits_subview_loading: bool,
    commits_subview_source: Option<String>,
    commits_subview: CommitsRenderModel,
}

impl BranchesPanelState {
    pub fn render_view(&self) -> BranchesRenderView<'_> {
        BranchesRenderView {
            panel: self.panel.view(),
            items: &self.items,
            commits_subview_active: self.commits_subview_active,
            commits_subview_loading: self.commits_subview_loading,
            commits_subview_source: self.commits_subview_source.as_ref(),
            commits_subview: self.commits_subview.render_view(),
        }
    }

    pub fn render_model(&self) -> BranchesRenderModel {
        BranchesRenderModel {
            panel: self.panel.view(),
            items: self.items.clone(),
            commits_subview_active: self.commits_subview_active,
            commits_subview_loading: self.commits_subview_loading,
            commits_subview_source: self.commits_subview_source.clone(),
            commits_subview: self.commits_subview.render_model(),
        }
    }
}

impl BranchesRenderModel {
    pub fn view(&self) -> BranchesRenderView<'_> {
        BranchesRenderView {
            panel: self.panel,
            items: &self.items,
            commits_subview_active: self.commits_subview_active,
            commits_subview_loading: self.commits_subview_loading,
            commits_subview_source: self.commits_subview_source.as_ref(),
            commits_subview: self.commits_subview.view(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct StashRenderView<'a> {
    pub panel: PanelView,
    pub items: &'a [StashInfo],
    pub tree_mode: RevisionTreeRenderView<'a, Option<&'a usize>>,
}

pub struct StashRenderModel {
    panel: PanelView,
    items: Shared<Vec<StashInfo>>,
    tree_mode: RevisionTreeRenderModel<usize>,
}

impl StashPanelState {
    pub fn render_view(&self) -> StashRenderView<'_> {
        StashRenderView {
            panel: self.panel.view(),
            items: &self.items,
            tree_mode: self.tree_mode.render_view(),
        }
    }

    pub fn render_model(&self) -> StashRenderModel {
        StashRenderModel {
            panel: self.panel.view(),
            items: self.items.clone(),
            tree_mode: (&self.tree_mode).into(),
        }
    }
}

impl StashRenderModel {
    pub fn view(&self) -> StashRenderView<'_> {
        StashRenderView {
            panel: self.panel,
            items: &self.items,
            tree_mode: self.tree_mode.view(),
        }
    }
}
