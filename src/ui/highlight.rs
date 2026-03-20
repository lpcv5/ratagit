use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

pub fn highlighted_spans(text: &str, query: Option<&str>, base_style: Style) -> Vec<Span<'static>> {
    let Some(query) = query else {
        return vec![Span::styled(text.to_string(), base_style)];
    };
    if query.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let text_l = text.to_ascii_lowercase();
    let query_l = query.to_ascii_lowercase();
    if query_l.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let mut spans = Vec::new();
    let mut start = 0usize;
    while let Some(found) = text_l[start..].find(&query_l) {
        let from = start + found;
        if from > start {
            spans.push(Span::styled(text[start..from].to_string(), base_style));
        }
        let to = from + query_l.len();
        spans.push(Span::styled(
            text[from..to].to_string(),
            base_style
                .bg(Color::Rgb(97, 76, 0))
                .add_modifier(Modifier::BOLD),
        ));
        start = to;
    }
    if start < text.len() {
        spans.push(Span::styled(text[start..].to_string(), base_style));
    }

    spans
}
