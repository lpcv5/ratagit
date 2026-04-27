use ratatui::style::Style;

use crate::theme::RowRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: String,
    pub(crate) selected: bool,
    pub(crate) role: RowRole,
    pub(crate) spans: Option<Vec<PanelSpan>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelSpan {
    pub(crate) text: String,
    pub(crate) style: Style,
}

impl PanelLine {
    pub(crate) fn new(text: impl Into<String>, role: RowRole) -> Self {
        Self {
            text: text.into(),
            selected: false,
            role,
            spans: None,
        }
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn styled_spans(mut self, spans: Vec<PanelSpan>) -> Self {
        self.spans = Some(spans);
        self
    }
}
