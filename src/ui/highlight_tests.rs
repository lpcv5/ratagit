use super::*;
use pretty_assertions::assert_eq;
use ratatui::style::{Color, Modifier, Style};

fn test_base_style() -> Style {
    Style::default().fg(Color::Cyan)
}

fn highlight_style(base_style: Style) -> Style {
    base_style
        .bg(Color::Rgb(97, 76, 0))
        .add_modifier(Modifier::BOLD)
}

fn span_descriptors(spans: Vec<Span<'static>>) -> Vec<(String, Style)> {
    spans
        .into_iter()
        .map(|span| (span.content.to_string(), span.style))
        .collect()
}

#[test]
fn highlighted_spans_missing_query_returns_single_plain_span() {
    let base_style = test_base_style();

    let spans = highlighted_spans("hello world", None, base_style);

    assert_eq!(
        span_descriptors(spans),
        vec![("hello world".to_string(), base_style)]
    );
}

#[test]
fn highlighted_spans_empty_query_returns_single_plain_span() {
    let base_style = test_base_style();

    let spans = highlighted_spans("hello world", Some(""), base_style);

    assert_eq!(
        span_descriptors(spans),
        vec![("hello world".to_string(), base_style)]
    );
}

#[test]
fn highlighted_spans_case_insensitive_query_highlights_match_and_keeps_original_case() {
    let base_style = test_base_style();

    let spans = highlighted_spans("Hello World", Some("hello"), base_style);

    assert_eq!(
        span_descriptors(spans),
        vec![
            ("Hello".to_string(), highlight_style(base_style)),
            (" World".to_string(), base_style),
        ]
    );
}

#[test]
fn highlighted_spans_multiple_matches_produces_all_highlighted_segments_in_order() {
    let base_style = test_base_style();

    let spans = highlighted_spans("abcabcx", Some("abc"), base_style);

    assert_eq!(
        span_descriptors(spans),
        vec![
            ("abc".to_string(), highlight_style(base_style)),
            ("abc".to_string(), highlight_style(base_style)),
            ("x".to_string(), base_style),
        ]
    );
}

#[test]
fn highlighted_spans_no_match_returns_single_plain_span() {
    let base_style = test_base_style();

    let spans = highlighted_spans("hello", Some("xyz"), base_style);

    assert_eq!(
        span_descriptors(spans),
        vec![("hello".to_string(), base_style)]
    );
}

#[test]
fn highlighted_spans_query_longer_than_text_returns_single_plain_span() {
    let base_style = test_base_style();

    let spans = highlighted_spans("abc", Some("abcde"), base_style);

    assert_eq!(
        span_descriptors(spans),
        vec![("abc".to_string(), base_style)]
    );
}

#[test]
fn highlighted_spans_empty_text_returns_no_spans() {
    let spans = highlighted_spans("", Some("abc"), test_base_style());

    assert_eq!(spans, Vec::new());
}
