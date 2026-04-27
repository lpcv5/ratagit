use ratagit_core::{AppContext, PanelFocus};
use ratatui::backend::{Backend, TestBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
};
use ratatui::{Frame, Terminal};

use crate::branch_modal::render_branch_modals;
use crate::discard_modal::render_discard_modal;
use crate::editor_modal::render_editor_modal;
use crate::frame::{RenderContext, TerminalBuffer, TerminalCursor, TerminalSize, buffer_to_text};
use crate::layout::compute_left_panel_heights;
use crate::loading_indicator::loading_indicator_for_state;
use crate::panel_projection::{PanelProjection, project_panel};
use crate::panels::{PanelLine, ShortcutLine, shortcut_line_for_state};
use crate::reset_modal::render_reset_modal;
use crate::sync_modal::render_sync_modal;
use crate::theme::{
    LoadingSpotlightTone, RowRole, batch_selected_row_style, focused_panel_style,
    inactive_panel_style, loading_spinner_style, loading_text_style, row_style, selected_row_style,
    title_badge_style,
};

pub fn render_terminal(frame: &mut Frame<'_>, state: &AppContext) {
    render_terminal_with_context(frame, state, RenderContext::default());
}

pub fn render_terminal_with_context(
    frame: &mut Frame<'_>,
    state: &AppContext,
    context: RenderContext,
) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);

    render_panel_grid(frame, state, root[0]);
    render_shortcuts(frame, state, context, root[1]);
    render_editor_modal(frame, state, root[0]);
    render_branch_modals(frame, state, root[0]);
    render_reset_modal(frame, state, root[0]);
    render_discard_modal(frame, state, root[0]);
    render_sync_modal(frame, state, root[0]);
}

pub fn render_terminal_text(state: &AppContext, size: TerminalSize) -> String {
    buffer_to_text(&render_terminal_buffer(state, size))
}

pub fn render_terminal_text_with_context(
    state: &AppContext,
    size: TerminalSize,
    context: RenderContext,
) -> String {
    buffer_to_text(&render_terminal_buffer_with_render_context(
        state, size, context,
    ))
}

pub fn render_terminal_buffer(state: &AppContext, size: TerminalSize) -> TerminalBuffer {
    render_terminal_buffer_with_cursor(state, size).0
}

pub fn render_terminal_buffer_with_render_context(
    state: &AppContext,
    size: TerminalSize,
    context: RenderContext,
) -> TerminalBuffer {
    render_terminal_buffer_with_cursor_and_context(state, size, context).0
}

pub fn render_terminal_buffer_with_cursor(
    state: &AppContext,
    size: TerminalSize,
) -> (TerminalBuffer, Option<TerminalCursor>) {
    render_terminal_buffer_with_cursor_and_context(state, size, RenderContext::default())
}

pub fn render_terminal_buffer_with_cursor_and_context(
    state: &AppContext,
    size: TerminalSize,
    context: RenderContext,
) -> (TerminalBuffer, Option<TerminalCursor>) {
    let backend = TestBackend::new(size.width.max(1) as u16, size.height.max(1) as u16);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| render_terminal_with_context(frame, state, context))
        .expect("terminal render should succeed");
    let cursor = if state.ui.editor.is_active() || state.ui.branches.create.active {
        let position = terminal
            .backend_mut()
            .get_cursor_position()
            .expect("test backend should expose cursor position");
        Some(TerminalCursor {
            x: position.x,
            y: position.y,
        })
    } else {
        None
    };
    (terminal.backend().buffer().clone(), cursor)
}

fn render_panel_grid(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(area);
    let left_heights = compute_left_panel_heights(state, columns[0].height as usize, 2);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(left_heights.files as u16),
            Constraint::Length(left_heights.branches as u16),
            Constraint::Length(left_heights.commits as u16),
            Constraint::Length(left_heights.stash as u16),
        ])
        .split(columns[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(columns[1]);

    render_block_panel(
        frame,
        left[0],
        project_panel(
            state,
            PanelFocus::Files,
            left[0].height.saturating_sub(2) as usize,
        ),
    );
    render_block_panel(
        frame,
        left[1],
        project_panel(
            state,
            PanelFocus::Branches,
            left[1].height.saturating_sub(2) as usize,
        ),
    );
    render_block_panel(
        frame,
        left[2],
        project_panel(
            state,
            PanelFocus::Commits,
            left[2].height.saturating_sub(2) as usize,
        ),
    );
    render_block_panel(
        frame,
        left[3],
        project_panel(
            state,
            PanelFocus::Stash,
            left[3].height.saturating_sub(2) as usize,
        ),
    );
    render_block_panel(
        frame,
        right[0],
        project_panel(
            state,
            PanelFocus::Details,
            right[0].height.saturating_sub(2) as usize,
        ),
    );
    render_block_panel(
        frame,
        right[1],
        project_panel(
            state,
            PanelFocus::Log,
            right[1].height.saturating_sub(2) as usize,
        ),
    );
}

fn render_block_panel(frame: &mut Frame<'_>, area: Rect, projection: PanelProjection) {
    let panel = projection.panel;
    let focused = projection.focused;
    let border_style = if focused {
        focused_panel_style()
    } else {
        inactive_panel_style()
    };
    let title = panel_title_line(&projection, border_style);
    let items = projection
        .lines
        .iter()
        .map(|line| ListItem::new(line_to_ratatui_line(line)).style(row_style(line.role)))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    let selected_index = projection.lines.iter().position(|line| line.selected);
    if focused && let Some(index) = selected_index {
        list_state.select(Some(index));
    }
    let highlight_style = if focused
        && selected_index
            .and_then(|index| projection.lines.get(index))
            .is_some_and(|line| line.role == RowRole::BatchSelected)
    {
        batch_selected_row_style()
    } else {
        selected_row_style()
    };
    let widget = List::new(items)
        .highlight_style(highlight_style)
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .title(title)
                .borders(panel_borders(panel, focused))
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        );
    frame.render_stateful_widget(widget, area, &mut list_state);
}

fn panel_borders(panel: PanelFocus, focused: bool) -> Borders {
    if focused {
        return Borders::ALL;
    }

    match panel {
        PanelFocus::Files => Borders::ALL,
        PanelFocus::Branches | PanelFocus::Commits | PanelFocus::Stash => {
            Borders::LEFT | Borders::RIGHT | Borders::BOTTOM
        }
        PanelFocus::Details => Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        PanelFocus::Log => Borders::RIGHT | Borders::BOTTOM,
    }
}

fn panel_title_line(
    projection: &PanelProjection,
    title_style: ratatui::style::Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {} ", projection.title.badge),
            title_badge_style(projection.focused),
        ),
        Span::styled(format!(" {} ", projection.title.body), title_style),
    ])
}

fn line_to_ratatui_line(line: &PanelLine) -> Line<'static> {
    Line::from(
        line.spans
            .iter()
            .map(|span| Span::styled(span.text.clone(), span.style))
            .collect::<Vec<_>>(),
    )
}

fn render_shortcuts(frame: &mut Frame<'_>, state: &AppContext, context: RenderContext, area: Rect) {
    let widget = Paragraph::new(shortcut_line_to_ratatui_line(
        state,
        context,
        shortcut_line_for_state(state),
    ));
    frame.render_widget(widget, area);
}

fn shortcut_line_to_ratatui_line(
    state: &AppContext,
    context: RenderContext,
    line: ShortcutLine,
) -> Line<'static> {
    let mut prefix =
        loading_indicator_for_state(state, context).map_or_else(Vec::new, |indicator| {
            let mut spans = vec![
                Span::styled(indicator.spinner, loading_spinner_style()),
                Span::raw(" "),
            ];
            spans.extend(loading_text_spans(
                &format!("loading: {}", indicator.kind),
                indicator.spotlight_index,
            ));
            spans.push(Span::raw("  "));
            spans
        });
    match line {
        ShortcutLine::Text(text) => {
            prefix.push(Span::raw(text));
            Line::from(prefix)
        }
        ShortcutLine::Segments(segments) => {
            let mut spans = prefix;
            for (index, segment) in segments.iter().enumerate() {
                if index > 0 {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    format!(" {} ", segment.key),
                    title_badge_style(true),
                ));
                spans.push(Span::raw(format!(" {}", segment.label)));
            }
            Line::from(spans)
        }
    }
}

fn loading_text_spans(text: &str, spotlight_index: usize) -> Vec<Span<'static>> {
    text.chars()
        .enumerate()
        .map(|(index, ch)| {
            Span::styled(
                ch.to_string(),
                loading_text_style(spotlight_tone(index, spotlight_index)),
            )
        })
        .collect()
}

fn spotlight_tone(index: usize, spotlight_index: usize) -> LoadingSpotlightTone {
    let distance = index.abs_diff(spotlight_index);
    match distance {
        0 => LoadingSpotlightTone::Bright,
        1 => LoadingSpotlightTone::Mid,
        _ => LoadingSpotlightTone::Dim,
    }
}
