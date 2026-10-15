use crate::app::InputMode;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::components::atoms::{badge, Tone};
use crate::ui::components::molecules::card;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_command_log(frame: &mut Frame, area: Rect, snapshot: &AppStateSnapshot<'_>) {
    let theme = UiTheme::default();
    let mut lines: Vec<Line> = Vec::new();

    if snapshot.input_mode == Some(InputMode::CreateBranch) {
        lines.push(Line::from(Span::styled(
            format!("branch> {}", snapshot.input_buffer),
            Style::default().fg(theme.warning),
        )));
    }

    if snapshot.command_log.is_empty() {
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No commands yet",
                Style::default().fg(theme.text_muted),
            )));
        }
    } else {
        let available = usize::from(area.height.saturating_sub(2)).saturating_sub(lines.len());
        lines.extend(
            snapshot
                .command_log
                .iter()
                .rev()
                .take(available)
                .map(|entry| {
                    let (label, tone) = if entry.success {
                        ("OK", Tone::Success)
                    } else {
                        ("ERR", Tone::Danger)
                    };
                    Line::from(vec![
                        badge(label, tone, &theme),
                        Span::styled(
                            format!(" {}", entry.command),
                            Style::default().fg(theme.text_primary),
                        ),
                    ])
                }),
        );
    }

    let paragraph = Paragraph::new(lines).block(card(&theme, "Command Log", false));

    frame.render_widget(paragraph, area);
}
