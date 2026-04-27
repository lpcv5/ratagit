use ratatui::style::Style;

use crate::theme::RowRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) spans: Vec<PanelSpan>,
    pub(crate) selected: bool,
    pub(crate) role: RowRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelSpan {
    pub(crate) text: String,
    pub(crate) style: Style,
}

impl PanelLine {
    pub(crate) fn new(text: impl Into<String>, role: RowRole) -> Self {
        Self {
            spans: vec![PanelSpan {
                text: text.into(),
                style: Style::default(),
            }],
            selected: false,
            role,
        }
    }

    pub(crate) fn from_spans(spans: Vec<PanelSpan>, role: RowRole) -> Self {
        Self {
            spans,
            selected: false,
            role,
        }
    }

    pub(crate) fn text(&self) -> String {
        plain_text(&self.spans)
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

pub(crate) fn plain_text(spans: &[PanelSpan]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    #[test]
    fn panel_line_plain_text_is_derived_from_spans() {
        let line = PanelLine::from_spans(
            vec![
                PanelSpan {
                    text: "hello".to_string(),
                    style: Style::default().fg(Color::Green),
                },
                PanelSpan {
                    text: " world".to_string(),
                    style: Style::default().fg(Color::Red),
                },
            ],
            RowRole::Normal,
        );

        assert_eq!(line.text(), "hello world");
    }

    #[test]
    fn panel_line_new_stores_plain_text_as_default_span() {
        let line = PanelLine::new("plain", RowRole::Muted);

        assert_eq!(line.text(), "plain");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].text, "plain");
        assert_eq!(line.spans[0].style, Style::default());
    }
}
