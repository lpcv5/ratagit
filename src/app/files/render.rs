use super::FilesPanelState;
use crate::app::PanelView;
use crate::file_tree::FileTreeNode;
use crate::shared::Shared;

/// Borrowed feature data; exposes no reducer or widget mutation API.
#[derive(Clone, Copy)]
pub struct FilesRenderView<'a> {
    pub panel: PanelView,
    pub tree_nodes: &'a [FileTreeNode],
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,
}

/// Published feature data shares immutable payloads and owns cursor metadata.
pub struct FilesRenderModel {
    panel: PanelView,
    tree_nodes: Shared<Vec<FileTreeNode>>,
    visual_mode: bool,
    visual_anchor: Option<usize>,
}

impl FilesPanelState {
    pub fn render_view(&self) -> FilesRenderView<'_> {
        FilesRenderView {
            panel: self.panel.view(),
            tree_nodes: &self.tree_nodes,
            visual_mode: self.visual_mode,
            visual_anchor: self.visual_anchor,
        }
    }

    pub fn render_model(&self) -> FilesRenderModel {
        FilesRenderModel {
            panel: self.panel.view(),
            tree_nodes: self.tree_nodes.clone(),
            visual_mode: self.visual_mode,
            visual_anchor: self.visual_anchor,
        }
    }
}

impl FilesRenderModel {
    pub fn view(&self) -> FilesRenderView<'_> {
        FilesRenderView {
            panel: self.panel,
            tree_nodes: &self.tree_nodes,
            visual_mode: self.visual_mode,
            visual_anchor: self.visual_anchor,
        }
    }
}
