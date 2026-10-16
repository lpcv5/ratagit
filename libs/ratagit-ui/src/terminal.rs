use ratagit_core::{AppContext, PanelFocus};
use ratatui::backend::{Backend, TestBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
};
use ratatui::{Frame, Terminal};

use crate::branch_modal::render_branch_modals;
use crate::command_palette_modal::render_command_palette_modal;
use crate::discard_modal::render_discard_modal;
use crate::editor_modal::render_editor_modal;
use crate::frame::{RenderContext, TerminalBuffer, TerminalCursor, TerminalSize, buffer_to_text};
use crate::layout::{compute_left_panel_heights, compute_right_panel_heights};
use crate::loading_indicator::loading_indicator_for_state;
use crate::panel_projection::{PanelProjection, project_panel_with_theme};
use crate::panels::{PanelLine, ShortcutLine, shortcut_line_for_state};
use crate::reset_modal::render_reset_modal;
use crate::stage_all_modal::render_stage_all_modal;
use crate::sync_modal::render_sync_modal;
use crate::theme::{LoadingSpotlightTone, RowRole, UiTheme};

pub fn render_terminal(frame: &mut Frame<'_>, state: &AppContext) {
    render_terminal_with_context(frame, state, RenderContext::default());
}

pub fn render_terminal_with_context(
    frame: &mut Frame<'_>,
    state: &AppContext,
    context: RenderContext,
) {
    let area = frame.area();
    frame.render_widget(Block::default().style(context.theme.base_style()), area);
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);

    render_panel_grid(frame, state, root[0], context.theme);
    render_shortcuts(frame, state, context, root[1]);
    render_editor_modal(frame, state, root[0], context.theme);
    render_branch_modals(frame, state, root[0], context.theme);
    render_reset_modal(frame, state, root[0], context.theme);
    render_discard_modal(frame, state, root[0], context.theme);
    render_sync_modal(frame, state, root[0], context.theme);
    render_stage_all_modal(frame, state, root[0], context.theme);
    render_command_palette_modal(frame, state, root[0], context.theme);
    crate::workflow_modal::render(frame, state, root[0], context.theme);
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
    let cursor = if state.ui.editor.is_active()
        || state.ui.branches.create.active
        || matches!(
            state.ui.workflow.dialog,
            Some(ratagit_core::workflow::Dialog::Prompt { .. })
        ) {
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

fn render_panel_grid(frame: &mut Frame<'_>, state: &AppContext, area: Rect, theme: &UiTheme) {
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
    let right_heights = compute_right_panel_heights(columns[1].height as usize, 2);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints(right_heights.map(|height| Constraint::Length(height as u16)))
        .split(columns[1]);

    render_block_panel(
        frame,
        left[0],
        project_panel_with_theme(
            state,
            PanelFocus::Files,
            left[0].height.saturating_sub(2) as usize,
            theme,
        ),
        theme,
    );
    render_block_panel(
        frame,
        left[1],
        project_panel_with_theme(
            state,
            PanelFocus::Branches,
            left[1].height.saturating_sub(2) as usize,
            theme,
        ),
        theme,
    );
    render_block_panel(
        frame,
        left[2],
        project_panel_with_theme(
            state,
            PanelFocus::Commits,
            left[2].height.saturating_sub(2) as usize,
            theme,
        ),
        theme,
    );
    render_block_panel(
        frame,
        left[3],
        project_panel_with_theme(
            state,
            PanelFocus::Stash,
            left[3].height.saturating_sub(2) as usize,
            theme,
        ),
        theme,
    );
    render_block_panel(
        frame,
        right[0],
        project_panel_with_theme(
            state,
            PanelFocus::Details,
            right[0].height.saturating_sub(2) as usize,
            theme,
        ),
        theme,
    );
    render_block_panel(
        frame,
        right[1],
        project_panel_with_theme(
            state,
            PanelFocus::Log,
            right[1].height.saturating_sub(2) as usize,
            theme,
        ),
        theme,
    );
}

fn render_block_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    projection: PanelProjection,
    theme: &UiTheme,
) {
    let focused = projection.focused;
    let border_style = if focused {
        theme.focused_panel_style()
    } else {
        theme.inactive_panel_style()
    };
    let title = panel_title_line(&projection, border_style, theme);
    let items = projection
        .lines
        .iter()
        .map(|line| {
            ListItem::new(line_to_ratatui_line(line, theme))
                .style(theme.row_style(line.role).patch(line.style))
        })
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
        theme.batch_selected_row_style()
    } else {
        theme.selected_row_style()
    };
    let widget = List::new(items)
        .highlight_style(highlight_style)
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        );
    frame.render_stateful_widget(widget, area, &mut list_state);
}

fn panel_title_line(
    projection: &PanelProjection,
    title_style: ratatui::style::Style,
    theme: &UiTheme,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {} ", projection.title.badge),
            theme.title_badge_style(projection.focused),
        ),
        Span::styled(format!(" {} ", projection.title.body), title_style),
    ])
}

fn line_to_ratatui_line(line: &PanelLine, theme: &UiTheme) -> Line<'static> {
    let overlay = if line.role == RowRole::BatchSelected {
        theme.batch_selected_row_style()
    } else {
        ratatui::style::Style::default()
    };
    Line::from(
        line.spans
            .iter()
            .map(|span| Span::styled(span.text.clone(), span.style.patch(overlay)))
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
                Span::styled(indicator.spinner, context.theme.loading_spinner_style()),
                Span::raw(" "),
            ];
            spans.extend(loading_text_spans(
                &format!("loading: {}", indicator.kind),
                indicator.spotlight_index,
                context.theme,
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
                    context.theme.title_badge_style(true),
                ));
                spans.push(Span::raw(format!(" {}", segment.label)));
            }
            Line::from(spans)
        }
    }
}

fn loading_text_spans(text: &str, spotlight_index: usize, theme: &UiTheme) -> Vec<Span<'static>> {
    text.chars()
        .enumerate()
        .map(|(index, ch)| {
            Span::styled(
                ch.to_string(),
                theme.loading_text_style(spotlight_tone(index, spotlight_index)),
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
