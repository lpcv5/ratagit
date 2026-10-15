//! Controlled side-panel inputs. View choice, focus and graph ownership are
//! resolved by the application adapter before components receive these props.
use crate::file_tree::FileTreeNode;
use crate::ui::components::atoms::Tone;
use std::collections::HashSet;

pub struct PanelHeader<'a> {
    pub title: String,
    pub is_active: bool,
    pub selected: Option<usize>,
    pub search_query: Option<&'a str>,
}

pub struct FilesPanelProps<'a> {
    pub header: PanelHeader<'a>,
    pub nodes: &'a [FileTreeNode],
    pub selected_indices: &'a HashSet<usize>,
}

pub struct BranchRowProps<'a> {
    pub name: &'a str,
    pub is_current: bool,
}

pub enum BranchesPanelProps<'a> {
    List {
        header: PanelHeader<'a>,
        rows: Vec<BranchRowProps<'a>>,
    },
    Commits {
        props: CommitsPanelProps<'a>,
        view_id: Option<&'a str>,
    },
}

pub struct GraphCellProps<'a> {
    pub text: &'a str,
    pub dimmed: bool,
}

pub struct CommitRowProps<'a> {
    pub oid: &'a str,
    pub author: &'a str,
    pub message: &'a str,
    pub tone: Tone,
    pub graph: Vec<GraphCellProps<'a>>,
}

/// A display variant and an opaque viewport identity, with no domain tree mode.
pub enum RevisionContent<'a, Rows> {
    List(Rows),
    Tree {
        nodes: &'a [FileTreeNode],
        view_id: Option<String>,
    },
}

impl<'a, Rows> RevisionContent<'a, Rows> {
    pub fn map<T>(self, render_rows: impl FnOnce(Rows) -> T) -> RevisionContent<'a, T> {
        match self {
            Self::List(rows) => RevisionContent::List(render_rows(rows)),
            Self::Tree { nodes, view_id } => RevisionContent::Tree { nodes, view_id },
        }
    }
}

pub struct CommitsPanelProps<'a> {
    pub header: PanelHeader<'a>,
    pub content: RevisionContent<'a, Vec<CommitRowProps<'a>>>,
}

pub struct StashRowProps<'a> {
    pub index: usize,
    pub message: &'a str,
}

pub struct StashPanelProps<'a> {
    pub header: PanelHeader<'a>,
    pub content: RevisionContent<'a, Vec<StashRowProps<'a>>>,
}

pub struct SidePanelLayoutProps {
    pub stash_focused: bool,
    pub focus_index: Option<usize>,
    pub focus_item_count: usize,
}
