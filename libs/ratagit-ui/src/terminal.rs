use ratagit_core::{AppState, PanelFocus};
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
use crate::frame::{TerminalBuffer, TerminalCursor, TerminalSize, buffer_to_text};
use crate::layout::compute_left_panel_heights;
use crate::panels::{
    PanelLine, ShortcutLine, panel_title_label, render_branches_lines, render_commits_lines,
    render_details_lines, render_files_lines, render_log_lines, render_stash_lines,
    shortcut_line_for_state,
};
use crate::reset_modal::render_reset_modal;
use crate::theme::{
    RowRole, batch_selected_row_style, focused_panel_style, inactive_panel_style, row_style,
    selected_row_style, title_badge_style,
};

pub fn render_terminal(frame: &mut Frame<'_>, state: &AppState) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);

    render_panel_grid(frame, state, root[0]);
    render_shortcuts(frame, state, root[1]);
    render_editor_modal(frame, state, root[0]);
    render_branch_modals(frame, state, root[0]);
    render_reset_modal(frame, state, root[0]);
    render_discard_modal(frame, state, root[0]);
}

pub fn render_terminal_text(state: &AppState, size: TerminalSize) -> String {
    buffer_to_text(&render_terminal_buffer(state, size))
}

pub fn render_terminal_buffer(state: &AppState, size: TerminalSize) -> TerminalBuffer {
    render_terminal_buffer_with_cursor(state, size).0
}

pub fn render_terminal_buffer_with_cursor(
    state: &AppState,
    size: TerminalSize,
) -> (TerminalBuffer, Option<TerminalCursor>) {
    let backend = TestBackend::new(size.width.max(1) as u16, size.height.max(1) as u16);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| render_terminal(frame, state))
        .expect("terminal render should succeed");
    let cursor = if state.editor.is_active() || state.branches.create.active {
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

fn render_panel_grid(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
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
        state,
        PanelFocus::Files,
        left[0],
        render_files_lines(state, left[0].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Branches,
        left[1],
        render_branches_lines(state, left[1].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Commits,
        left[2],
        render_commits_lines(state, left[2].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Stash,
        left[3],
        render_stash_lines(state, left[3].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Details,
        right[0],
        render_details_lines(state, right[0].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Log,
        right[1],
        render_log_lines(state, right[1].height.saturating_sub(2) as usize),
    );
}

fn render_block_panel(
    frame: &mut Frame<'_>,
    state: &AppState,
    panel: PanelFocus,
    area: Rect,
    lines: Vec<PanelLine>,
) {
    let focused = state.focus == panel;
    let border_style = if focused {
        focused_panel_style()
    } else {
        inactive_panel_style()
    };
    let title = panel_title_line(state, panel, focused, border_style);
    let items = lines
        .iter()
        .map(|line| ListItem::new(line_to_ratatui_line(line)).style(row_style(line.role)))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    let selected_index = lines.iter().position(|line| line.selected);
    if focused && let Some(index) = selected_index {
        list_state.select(Some(index));
    }
    let highlight_style = if focused
        && selected_index
            .and_then(|index| lines.get(index))
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
    state: &AppState,
    panel: PanelFocus,
    focused: bool,
    title_style: ratatui::style::Style,
) -> Line<'static> {
    let label = panel_title_label(state, panel);
    Line::from(vec![
        Span::styled(format!(" {} ", label.badge), title_badge_style(focused)),
        Span::styled(format!(" {} ", label.body), title_style),
    ])
}

fn line_to_ratatui_line(line: &PanelLine) -> Line<'static> {
    if let Some(spans) = &line.spans {
        return Line::from(
            spans
                .iter()
                .map(|span| Span::styled(span.text.clone(), span.style))
                .collect::<Vec<_>>(),
        );
    }
    Line::from(line.text.clone())
}

fn render_shortcuts(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let widget = Paragraph::new(shortcut_line_to_ratatui_line(shortcut_line_for_state(
        state,
    )));
    frame.render_widget(widget, area);
}

fn shortcut_line_to_ratatui_line(line: ShortcutLine) -> Line<'static> {
    match line {
        ShortcutLine::Text(text) => Line::from(text),
        ShortcutLine::Segments(segments) => {
            let mut spans = Vec::new();
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
