use std::collections::HashSet;

/// The four side panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SidePanel {
    #[default]
    Files,
    LocalBranches,
    Commits,
    Stash,
}

/// Domain cursor. Viewport offsets are calculated locally by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelState {
    selected: Option<usize>,
}

impl PanelState {
    pub fn new() -> Self {
        Self { selected: Some(0) }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, selected: Option<usize>) {
        self.selected = selected;
    }

    pub fn view(&self) -> PanelView {
        PanelView {
            selected: self.selected,
        }
    }
}

/// Read-only selection supplied to feature render models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelView {
    selected: Option<usize>,
}

impl PanelView {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
}

impl Default for PanelState {
    fn default() -> Self {
        Self::new()
    }
}

/// A single entry in the command/operation log shown in the UI.
#[derive(Clone)]
pub struct CommandLogEntry {
    pub command: String,
    pub success: bool,
}

/// Per-render cached values that are expensive to recompute each frame.
#[derive(Default, Clone)]
pub struct RenderCache {
    pub files_visual_selected_indices: crate::shared::Shared<HashSet<usize>>,
    pub files_search_summary: Option<String>,
    pub branches_search_summary: Option<String>,
    pub commits_search_summary: Option<String>,
    pub stash_search_summary: Option<String>,
}
