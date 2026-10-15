use crate::app::SidePanel;
use crate::git::{DiffLine, DiffLineKind};
use crate::ui::components::molecules::{card, EmptyState};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

pub struct DiffViewProps<'a> {
    pub hunks: crate::app::HunkView<'a>,
    pub lines: &'a [DiffLine],
    pub scroll: usize,
    pub active_panel: SidePanel,
    pub is_loading: bool,
}

#[derive(Default)]
pub struct DiffViewport {
    request: Option<crate::git::patch::PatchRequest>,
    selected: Option<usize>,
    requested_scroll: usize,
    offset: usize,
    manual: bool,
}

impl DiffViewport {
    fn offset(&mut self, props: &DiffViewProps<'_>, height: usize) -> usize {
        let patch = props.hunks.patch;
        let selected = patch
            .filter(|_| props.hunks.ready && props.hunks.interaction.focused)
            .and_then(|patch| {
                if props.hunks.interaction.line_mode {
                    props
                        .hunks
                        .interaction
                        .line_cursor
                        .and_then(|id| patch.line_row(id))
                } else {
                    patch
                        .hunk_spans()
                        .iter()
                        .find(|span| Some(span.id) == props.hunks.interaction.cursor)
                        .map(|span| span.range.start)
                }
            });
        let request = patch.map(|patch| &patch.request);
        if self.request.as_ref() != request {
            *self = Self {
                request: request.cloned(),
                ..Default::default()
            };
        }
        if selected.is_some() && selected != self.selected {
            self.manual = false;
        } else if props.scroll != self.requested_scroll {
            // Explicit page scrolling remains independent of cursor following.
            self.offset = props.scroll;
            self.manual = true;
        }
        if selected.is_some() && !self.manual {
            self.offset =
                super::selection::scroll_offset(self.offset, selected, props.lines.len(), height);
        } else {
            self.offset = self.offset.min(props.lines.len().saturating_sub(1));
        }
        self.selected = selected;
        self.requested_scroll = props.scroll;
        self.offset
    }
}

pub fn render_diff_panel(
    frame: &mut Frame,
    area: Rect,
    props: DiffViewProps<'_>,
    viewport: &mut DiffViewport,
) {
    let theme = UiTheme::default();

    if props.is_loading {
        *viewport = Default::default();
        let block = card(&theme, "Diff", props.hunks.interaction.focused);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(EmptyState::new("Loading diff...", &theme), inner);
        return;
    }

    if props.lines.is_empty() {
        *viewport = Default::default();
        let hint = match props.hunks.interaction.side {
            Some(crate::app::DiffSide::Unstaged) => "No unstaged changes",
            Some(crate::app::DiffSide::Staged) => "No staged changes",
            None => match props.active_panel {
                SidePanel::Files => "Select a file to view diff",
                SidePanel::LocalBranches => "Select a branch to view details",
                SidePanel::Commits => "Select a commit/file to view diff",
                SidePanel::Stash => "Select a stash entry/file to view diff",
            },
        };
        let block = card(&theme, "Diff", props.hunks.interaction.focused);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(EmptyState::new(hint, &theme), inner);
        return;
    }

    let visible_rows = usize::from(area.height.saturating_sub(2)).max(1);
    let scroll = viewport.offset(&props, visible_rows);
    let rows = props
        .hunks
        .patch
        .filter(|_| props.hunks.ready)
        .map(|patch| patch.hunk_spans())
        .unwrap_or_default();
    let marked_lines = props.hunks.marked_lines();
    let marked_hunks = props.hunks.marked_hunks();
    let items: Vec<ListItem> = props
        .lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_rows)
        .map(|(index, line)| {
            if matches!(line.kind, DiffLineKind::Context) && line.content.contains('\u{1b}') {
                return ListItem::new(parse_ansi_line(&line.content));
            }

            let (mut style, prefix) = match line.kind {
                DiffLineKind::Added => (
                    Style::default().fg(theme.success).bg(theme.diff_added_bg),
                    "+",
                ),
                DiffLineKind::Removed => (
                    Style::default().fg(theme.error).bg(theme.diff_removed_bg),
                    "-",
                ),
                DiffLineKind::Header => (
                    Style::default().fg(theme.accent).bg(theme.surface_raised),
                    "",
                ),
                DiffLineKind::Context => (Style::default().fg(theme.text_primary), " "),
            };
            let span = rows.iter().find(|span| span.range.contains(&index));
            let hunk = span.map(|span| span.id);
            let line_id = span
                .filter(|span| {
                    index > span.range.start
                        && matches!(
                            line.kind,
                            DiffLineKind::Added | DiffLineKind::Removed | DiffLineKind::Context
                        )
                })
                .map(|span| crate::git::patch::LineId {
                    file: span.id.file,
                    hunk: span.id.hunk,
                    line: index - span.range.start - 1,
                });
            let cursor = if props.hunks.interaction.line_mode {
                line_id.is_some() && line_id == props.hunks.interaction.line_cursor
            } else {
                hunk.is_some() && hunk == props.hunks.interaction.cursor
            };
            let selected = if props.hunks.interaction.line_mode {
                line_id.is_some_and(|id| marked_lines.contains(&id))
            } else {
                hunk.is_some_and(|id| marked_hunks.contains(&id))
            };
            let mut marker = String::new();
            if props.hunks.interaction.focused {
                if cursor {
                    style = style.bg(theme.selection_bg);
                }
                if selected {
                    style = style.add_modifier(ratatui::style::Modifier::BOLD);
                }
                marker = format!(
                    "{}{} ",
                    if cursor { ">" } else { " " },
                    if selected { "[x]" } else { "[ ]" }
                );
                if props.hunks.interaction.line_mode && line_id.is_none() {
                    marker = "     ".into();
                }
            }
            let source = if span.is_some_and(|span| span.range.start == index) {
                hunk.and_then(|id| props.hunks.patch.map(|patch| &patch.files[id.file].source))
                    .map(|source| match source {
                        crate::git::patch::PatchSource::Worktree
                        | crate::git::patch::PatchSource::Untracked => "Unstaged ",
                        crate::git::patch::PatchSource::Index => "Staged ",
                        _ => "",
                    })
                    .unwrap_or_default()
            } else {
                ""
            };
            let text = Line::from(vec![Span::styled(
                format!(
                    "{}{}{}{}",
                    marker,
                    source,
                    prefix,
                    line.content.trim_start_matches('\n')
                ),
                style,
            )]);
            ListItem::new(text).style(style)
        })
        .collect();

    let total = props.lines.len();
    let end = (scroll + items.len()).min(total);
    let ids = props.hunks.ids();
    let position = props
        .hunks
        .interaction
        .cursor
        .and_then(|cursor| ids.iter().position(|id| *id == cursor))
        .map(|i| i + 1)
        .unwrap_or(0);
    let status = if props.hunks.interaction.pending_write.is_some() {
        if props.hunks.interaction.line_mode {
            " | Applying lines"
        } else {
            " | Applying hunks"
        }
    } else if props.hunks.patch.is_some_and(|patch| {
        props.hunks.interaction.cursor.is_some_and(|id| {
            patch
                .validate_file_hunks(
                    id.file,
                    &[id.hunk],
                    crate::git::patch::PatchDirection::Stage,
                )
                .is_err()
                && patch
                    .validate_file_hunks(
                        id.file,
                        &[id.hunk],
                        crate::git::patch::PatchDirection::Unstage,
                    )
                    .is_err()
        })
    }) {
        " | Read only"
    } else {
        ""
    };
    let title = if props.hunks.interaction.focused {
        let path = props
            .hunks
            .interaction
            .cursor
            .and_then(|id| props.hunks.patch.and_then(|patch| patch.files.get(id.file)))
            .and_then(|file| file.new.path.as_ref().or(file.old.path.as_ref()))
            .map(|path| format!(" | {}", path.display()))
            .unwrap_or_default();
        if props.hunks.interaction.line_mode {
            let lines = props.hunks.line_ids();
            let position = props
                .hunks
                .interaction
                .line_cursor
                .and_then(|cursor| lines.iter().position(|id| *id == cursor))
                .map(|i| i + 1)
                .unwrap_or(0);
            let range = if props.hunks.interaction.range_anchor.is_some() {
                " | Range"
            } else {
                ""
            };
            format!(
                "Diff [Line {position}/{} | {} selected{range}{status}{path}]",
                lines.len(),
                marked_lines.len()
            )
        } else {
            format!(
                "Diff [Hunk {position}/{} | {} selected{status}{path}]",
                ids.len(),
                marked_hunks.len()
            )
        }
    } else {
        format!("Diff [{}-{} / {}]{status}", scroll + 1, end, total)
    };

    let list = List::new(items).block(card(&theme, &title, props.hunks.interaction.focused));

    frame.render_widget(list, area);
}

fn parse_ansi_line(input: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut chars = input.chars().peekable();
    let mut text = String::new();
    let mut style = Style::default();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            if !text.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut text), style));
            }

            let mut seq = String::new();
            while let Some(&c) = chars.peek() {
                let _ = chars.next();
                if c == 'm' {
                    break;
                }
                seq.push(c);
            }

            style = apply_sgr_codes(style, &seq);
            continue;
        }
        text.push(ch);
    }

    if !text.is_empty() {
        spans.push(Span::styled(text, style));
    }

    Line::from(spans)
}

fn apply_sgr_codes(mut style: Style, seq: &str) -> Style {
    let mut codes = seq
        .split(';')
        .filter_map(|part| part.parse::<u16>().ok())
        .peekable();

    if seq.is_empty() {
        return Style::default();
    }

    while let Some(code) = codes.next() {
        match code {
            0 => style = Style::default(),
            1 => style = style.add_modifier(ratatui::style::Modifier::BOLD),
            22 => style = style.remove_modifier(ratatui::style::Modifier::BOLD),
            30..=37 => style = style.fg(ansi_basic_color(code - 30, false)),
            90..=97 => style = style.fg(ansi_basic_color(code - 90, true)),
            39 => style = style.fg(Color::Reset),
            38 => {
                if let Some(mode) = codes.next() {
                    match mode {
                        5 => {
                            if let Some(idx) = codes.next() {
                                style = style.fg(Color::Indexed(idx as u8));
                            }
                        }
                        2 => {
                            let r = codes.next().unwrap_or(255) as u8;
                            let g = codes.next().unwrap_or(255) as u8;
                            let b = codes.next().unwrap_or(255) as u8;
                            style = style.fg(Color::Rgb(r, g, b));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    style
}

fn ansi_basic_color(index: u16, bright: bool) -> Color {
    match (index, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::Reset,
    }
}
