use ratagit_core::{AppState, ResetChoice};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::theme::{focused_panel_style, selected_row_style};

pub(crate) fn render_reset_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    if !state.reset_menu.active {
        return;
    }

    let modal = centered_rect(area, 72, 13);
    if modal.width < 20 || modal.height < 8 {
        return;
    }

    let block = Block::default()
        .title(" Reset ")
        .borders(Borders::ALL)
        .border_style(focused_panel_style());
    let inner = block.inner(modal);
    let content = inset_rect(inner, 1, 0);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(content);

    frame.render_widget(Clear, modal);
    frame.render_widget(block, modal);
    frame.render_widget(
        Paragraph::new("Choose reset scope for the whole repo."),
        rows[0],
    );

    let items = ResetChoice::ALL
        .iter()
        .map(|choice| ListItem::new(Line::from(format!("  {}", reset_choice_label(*choice)))))
        .collect::<Vec<_>>();
    let selected = ResetChoice::ALL
        .iter()
        .position(|choice| *choice == state.reset_menu.selected)
        .unwrap_or(0);
    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .highlight_style(selected_row_style())
        .highlight_spacing(HighlightSpacing::Never)
        .block(Block::default().title(" Mode ").borders(Borders::ALL));
    frame.render_stateful_widget(list, rows[1], &mut list_state);

    frame.render_widget(Paragraph::new("Description"), rows[2]);
    frame.render_widget(
        Paragraph::new(reset_choice_description(state.reset_menu.selected))
            .wrap(Wrap { trim: false }),
        rows[3],
    );
}

fn reset_choice_label(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Mixed => "mixed",
        ResetChoice::Soft => "soft",
        ResetChoice::Hard => "hard",
        ResetChoice::Nuke => "Nuke",
    }
}

fn reset_choice_description(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Mixed => "Mixed: reset index to HEAD; keep working tree changes.",
        ResetChoice::Soft => "Soft: move HEAD/index only; keep staged and working tree changes.",
        ResetChoice::Hard => "Hard: discard tracked working tree and index changes.",
        ResetChoice::Nuke => {
            "Nuke: hard reset, then remove untracked files/directories with `git clean -fd`."
        }
    }
}

fn inset_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    let shrink_x = horizontal.saturating_mul(2).min(area.width);
    let shrink_y = vertical.saturating_mul(2).min(area.height);
    Rect::new(
        area.x.saturating_add(horizontal.min(area.width)),
        area.y.saturating_add(vertical.min(area.height)),
        area.width.saturating_sub(shrink_x),
        area.height.saturating_sub(shrink_y),
    )
}

fn centered_rect(area: Rect, target_width: u16, target_height: u16) -> Rect {
    let max_width = area.width.saturating_sub(2).max(1);
    let max_height = area.height.saturating_sub(2).max(1);
    let width = target_width.min(max_width).max(20.min(area.width));
    let height = target_height.min(max_height).max(8.min(area.height));
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}
