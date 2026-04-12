use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    DefaultTerminal, Frame,
};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::backend::{BackendCommand, FrontendEvent};
use crate::components::Panel;
use crate::git::{BranchEntry, CommitEntry, StashEntry, StatusEntry};

pub struct App {
    cmd_tx: UnboundedSender<BackendCommand>,
    event_rx: UnboundedReceiver<FrontendEvent>,
    active_panel: Panel,
    files: Vec<StatusEntry>,
    branches: Vec<BranchEntry>,
    commits: Vec<CommitEntry>,
    stashes: Vec<StashEntry>,
    file_state: ListState,
    branch_state: ListState,
    commit_state: ListState,
    stash_state: ListState,
    main_view_title: String,
    main_view_content: String,
    main_scroll: u16,
    log_entries: Vec<String>,
    log_scroll: u16,
    pending_diff_path: Option<String>,
    should_quit: bool,
}

impl App {
    pub fn new(
        cmd_tx: UnboundedSender<BackendCommand>,
        event_rx: UnboundedReceiver<FrontendEvent>,
    ) -> Self {
        Self {
            cmd_tx,
            event_rx,
            active_panel: Panel::Files,
            files: Vec::new(),
            branches: Vec::new(),
            commits: Vec::new(),
            stashes: Vec::new(),
            file_state: default_list_state(),
            branch_state: default_list_state(),
            commit_state: default_list_state(),
            stash_state: default_list_state(),
            main_view_title: "Main View".to_string(),
            main_view_content: "Loading repository data...".to_string(),
            main_scroll: 0,
            log_entries: vec!["Application started".to_string()],
            log_scroll: 0,
            pending_diff_path: None,
            should_quit: false,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        self.request_refresh_all();
        self.update_main_view_for_active_panel()?;

        let result = self.main_loop(&mut terminal).await;
        ratatui::restore();
        result
    }

    async fn main_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            self.drain_backend_events().await?;
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(100))? {
                let input = event::read()?;
                self.handle_input(input)?;
            }
        }

        Ok(())
    }

    async fn drain_backend_events(&mut self) -> Result<()> {
        loop {
            match self.event_rx.try_recv() {
                Ok(event) => self.handle_backend_event(event)?,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.should_quit = true;
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') if key.modifiers.is_empty() => {
                    self.set_active_panel(self.active_panel.next())?;
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h')
                    if key.modifiers.is_empty() =>
                {
                    self.set_active_panel(self.active_panel.previous())?;
                }
                KeyCode::Char('r') if key.modifiers.is_empty() => self.request_refresh_all(),
                KeyCode::Enter => self.activate_panel()?,
                KeyCode::Char('j') | KeyCode::Down => self.navigate_forward()?,
                KeyCode::Char('k') | KeyCode::Up => self.navigate_backward()?,
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    self.scroll_active_panel(-5)
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    self.scroll_active_panel(5)
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_backend_event(&mut self, event: FrontendEvent) -> Result<()> {
        match event {
            FrontendEvent::StatusUpdated { files } => {
                self.files = files;
                sync_list_state(&mut self.file_state, self.files.len());
                self.push_log(format!("Files refreshed: {} entries", self.files.len()));

                if matches!(self.active_panel, Panel::Files | Panel::MainView) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::BranchesUpdated { branches } => {
                self.branches = branches;
                sync_list_state(&mut self.branch_state, self.branches.len());
                self.push_log(format!(
                    "Branches refreshed: {} entries",
                    self.branches.len()
                ));

                if matches!(self.active_panel, Panel::Branches | Panel::MainView) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::CommitsUpdated { commits } => {
                self.commits = commits;
                sync_list_state(&mut self.commit_state, self.commits.len());
                self.push_log(format!("Commits refreshed: {} entries", self.commits.len()));

                if matches!(self.active_panel, Panel::Commits | Panel::MainView) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::StashesUpdated { stashes } => {
                self.stashes = stashes;
                sync_list_state(&mut self.stash_state, self.stashes.len());
                self.push_log(format!("Stashes refreshed: {} entries", self.stashes.len()));

                if matches!(self.active_panel, Panel::Stash | Panel::MainView) {
                    self.update_main_view_for_active_panel()?;
                }
            }
            FrontendEvent::DiffLoaded { file_path, diff } => {
                self.pending_diff_path = None;
                self.main_view_title = format!("Main View · Diff · {file_path}");
                self.main_view_content = diff;
                self.main_scroll = 0;
                self.push_log(format!("Loaded diff for {file_path}"));
            }
            FrontendEvent::Error(message) => {
                self.main_view_title = "Main View · Error".to_string();
                self.main_view_content = message.clone();
                self.main_scroll = 0;
                self.pending_diff_path = None;
                self.push_log(format!("Error: {message}"));
            }
        }

        Ok(())
    }

    fn request_refresh_all(&self) {
        let _ = self.cmd_tx.send(BackendCommand::RefreshStatus);
        let _ = self.cmd_tx.send(BackendCommand::RefreshBranches);
        let _ = self
            .cmd_tx
            .send(BackendCommand::RefreshCommits { limit: 30 });
        let _ = self.cmd_tx.send(BackendCommand::RefreshStashes);
    }

    fn set_active_panel(&mut self, panel: Panel) -> Result<()> {
        self.active_panel = panel;
        self.push_log(format!("Focus moved to {}", panel.title()));
        self.update_main_view_for_active_panel()
    }

    fn activate_panel(&mut self) -> Result<()> {
        self.push_log(format!("Activated {}", self.active_panel.title()));
        self.update_main_view_for_active_panel()
    }

    fn navigate_forward(&mut self) -> Result<()> {
        match self.active_panel {
            Panel::Files => {
                cycle_selection(&mut self.file_state, self.files.len(), 1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::Branches => {
                cycle_selection(&mut self.branch_state, self.branches.len(), 1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::Commits => {
                cycle_selection(&mut self.commit_state, self.commits.len(), 1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::Stash => {
                cycle_selection(&mut self.stash_state, self.stashes.len(), 1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::MainView => self.main_scroll = self.main_scroll.saturating_add(1),
            Panel::Log => self.log_scroll = self.log_scroll.saturating_add(1),
        }

        Ok(())
    }

    fn navigate_backward(&mut self) -> Result<()> {
        match self.active_panel {
            Panel::Files => {
                cycle_selection(&mut self.file_state, self.files.len(), -1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::Branches => {
                cycle_selection(&mut self.branch_state, self.branches.len(), -1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::Commits => {
                cycle_selection(&mut self.commit_state, self.commits.len(), -1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::Stash => {
                cycle_selection(&mut self.stash_state, self.stashes.len(), -1);
                self.update_main_view_for_active_panel()?;
            }
            Panel::MainView => self.main_scroll = self.main_scroll.saturating_sub(1),
            Panel::Log => self.log_scroll = self.log_scroll.saturating_sub(1),
        }

        Ok(())
    }

    fn scroll_active_panel(&mut self, delta: i16) {
        match self.active_panel {
            Panel::MainView => self.main_scroll = apply_scroll(self.main_scroll, delta),
            Panel::Log => self.log_scroll = apply_scroll(self.log_scroll, delta),
            _ => {}
        }
    }

    fn update_main_view_for_active_panel(&mut self) -> Result<()> {
        match self.active_panel {
            Panel::MainView => self.show_repo_overview(),
            Panel::Files => self.request_selected_file_diff()?,
            Panel::Branches => self.show_branch_detail(),
            Panel::Commits => self.show_commit_detail(),
            Panel::Stash => self.show_stash_detail(),
            Panel::Log => self.show_log_detail(),
        }

        Ok(())
    }

    fn show_repo_overview(&mut self) {
        let current_branch = self
            .branches
            .iter()
            .find(|branch| branch.is_head)
            .map(|branch| branch.name.clone())
            .unwrap_or_else(|| "(detached)".to_string());

        let staged = self.files.iter().filter(|file| file.is_staged).count();
        let unstaged = self.files.iter().filter(|file| file.is_unstaged).count();

        self.main_view_title = "Main View · Overview".to_string();
        self.main_view_content = format!(
            "Repository snapshot\n\nCurrent branch: {current_branch}\nFiles: {} (staged: {staged}, unstaged: {unstaged})\nBranches: {}\nCommits loaded: {}\nStashes: {}\n\nNavigation\n- Tab / Shift+Tab or h/l: switch panel focus\n- j/k or arrows: move inside the focused panel\n- Enter: refresh the current panel detail\n- r: refresh all Git-backed panels\n- Ctrl+d / Ctrl+u: fast scroll in Main View and Log\n- q: quit",
            self.files.len(),
            self.branches.len(),
            self.commits.len(),
            self.stashes.len()
        );
        self.main_scroll = 0;
    }

    fn request_selected_file_diff(&mut self) -> Result<()> {
        if let Some(path) = self.selected_file().map(|file| file.path.clone()) {
            self.pending_diff_path = Some(path.clone());
            self.main_view_title = format!("Main View · Diff · {path}");
            self.main_view_content = format!("Loading diff for {path}...");
            self.main_scroll = 0;

            self.cmd_tx
                .send(BackendCommand::GetDiff { file_path: path })?;
        } else {
            self.main_view_title = "Main View · Files".to_string();
            self.main_view_content = "No file is currently selected.".to_string();
            self.main_scroll = 0;
        }

        Ok(())
    }

    fn show_branch_detail(&mut self) {
        if let Some((name, is_head, upstream)) = self
            .selected_branch()
            .map(|branch| (branch.name.clone(), branch.is_head, branch.upstream.clone()))
        {
            let upstream = upstream.unwrap_or_else(|| "(no upstream)".to_string());
            let head_status = if is_head { "yes" } else { "no" };

            self.main_view_title = format!("Main View · Branch · {name}");
            self.main_view_content = format!(
                "Branch detail\n\nName: {}\nChecked out: {head_status}\nUpstream: {upstream}\n\nThis panel is wired for navigation first. Branch actions can be layered on top of the same focus model later.",
                name
            );
        } else {
            self.main_view_title = "Main View · Branches".to_string();
            self.main_view_content = "No branches available.".to_string();
        }
        self.main_scroll = 0;
    }

    fn show_commit_detail(&mut self) {
        if let Some((short_id, id, author, timestamp, summary, body)) =
            self.selected_commit().map(|commit| {
                (
                    commit.short_id.clone(),
                    commit.id.clone(),
                    commit.author.clone(),
                    commit.timestamp,
                    commit.summary.clone(),
                    commit.body.clone(),
                )
            })
        {
            let body = body
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or("(no body)");

            self.main_view_title = format!("Main View · Commit · {short_id}");
            self.main_view_content = format!(
                "Commit detail\n\nCommit: {}\nAuthor: {}\nTimestamp: {}\nSummary: {}\n\nBody:\n{}",
                id, author, timestamp, summary, body
            );
        } else {
            self.main_view_title = "Main View · Commits".to_string();
            self.main_view_content = "No commits loaded.".to_string();
        }
        self.main_scroll = 0;
    }

    fn show_stash_detail(&mut self) {
        if let Some((index, id, message)) = self
            .selected_stash()
            .map(|stash| (stash.index, stash.id.clone(), stash.message.clone()))
        {
            self.main_view_title = format!("Main View · Stash · #{index}");
            self.main_view_content = format!(
                "Stash detail\n\nIndex: {}\nId: {}\nMessage: {}\n\nStash actions can be added later without changing the panel navigation shell.",
                index, id, message
            );
        } else {
            self.main_view_title = "Main View · Stash".to_string();
            self.main_view_content = "No stashes found in this repository.".to_string();
        }
        self.main_scroll = 0;
    }

    fn show_log_detail(&mut self) {
        self.main_view_title = "Main View · Log Help".to_string();
        self.main_view_content = "The Log panel records UI focus changes, refreshes, and backend responses.\n\nUse j/k or Ctrl+d / Ctrl+u while the Log panel is focused to scroll through recent messages.".to_string();
        self.main_scroll = 0;
    }

    fn selected_file(&self) -> Option<&StatusEntry> {
        self.file_state
            .selected()
            .and_then(|index| self.files.get(index))
    }

    fn selected_branch(&self) -> Option<&BranchEntry> {
        self.branch_state
            .selected()
            .and_then(|index| self.branches.get(index))
    }

    fn selected_commit(&self) -> Option<&CommitEntry> {
        self.commit_state
            .selected()
            .and_then(|index| self.commits.get(index))
    }

    fn selected_stash(&self) -> Option<&StashEntry> {
        self.stash_state
            .selected()
            .and_then(|index| self.stashes.get(index))
    }

    fn push_log(&mut self, entry: String) {
        self.log_entries.push(entry);
        if self.log_entries.len() > 200 {
            let overflow = self.log_entries.len() - 200;
            self.log_entries.drain(0..overflow);
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
            .split(frame.area());

        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(28),
                Constraint::Percentage(24),
                Constraint::Percentage(28),
                Constraint::Percentage(20),
            ])
            .split(columns[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12), Constraint::Length(8)])
            .split(columns[1]);

        self.render_files_panel(frame, left[0]);
        self.render_branches_panel(frame, left[1]);
        self.render_commits_panel(frame, left[2]);
        self.render_stash_panel(frame, left[3]);
        self.render_main_view(frame, right[0]);
        self.render_log_panel(frame, right[1]);
    }

    fn render_main_view(&self, frame: &mut Frame, area: Rect) {
        let block = panel_block(self.active_panel == Panel::MainView, &self.main_view_title);
        let paragraph = Paragraph::new(Text::from(self.main_view_content.clone()))
            .block(block)
            .scroll((self.main_scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_files_panel(&mut self, frame: &mut Frame, area: Rect) {
        let items = if self.files.is_empty() {
            vec![ListItem::new("No changed files")]
        } else {
            self.files
                .iter()
                .map(|file| {
                    let marker = match (file.is_staged, file.is_unstaged) {
                        (true, true) => "SU",
                        (true, false) => "S ",
                        (false, true) => "U ",
                        (false, false) => "  ",
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(format!("[{marker}] "), Style::default().fg(Color::DarkGray)),
                        Span::styled(file.path.clone(), file_style(file)),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(panel_block(self.active_panel == Panel::Files, "Files"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.file_state);
    }

    fn render_branches_panel(&mut self, frame: &mut Frame, area: Rect) {
        let items = if self.branches.is_empty() {
            vec![ListItem::new("No branches loaded")]
        } else {
            self.branches
                .iter()
                .map(|branch| {
                    let marker = if branch.is_head { "*" } else { " " };
                    let upstream = branch.upstream.as_deref().unwrap_or("-");
                    ListItem::new(format!("[{marker}] {}  ({upstream})", branch.name))
                })
                .collect()
        };

        let list = List::new(items)
            .block(panel_block(
                self.active_panel == Panel::Branches,
                "Branches",
            ))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.branch_state);
    }

    fn render_stash_panel(&mut self, frame: &mut Frame, area: Rect) {
        let items = if self.stashes.is_empty() {
            vec![ListItem::new("No stashes")]
        } else {
            self.stashes
                .iter()
                .map(|stash| ListItem::new(format!("#{} {}", stash.index, stash.message)))
                .collect()
        };

        let list = List::new(items)
            .block(panel_block(self.active_panel == Panel::Stash, "Stash"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.stash_state);
    }

    fn render_commits_panel(&mut self, frame: &mut Frame, area: Rect) {
        let items = if self.commits.is_empty() {
            vec![ListItem::new("No commits loaded")]
        } else {
            self.commits
                .iter()
                .map(|commit| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{} ", commit.short_id),
                            Style::default().fg(Color::LightBlue),
                        ),
                        Span::raw(commit.summary.clone()),
                    ]))
                })
                .collect()
        };

        let list = List::new(items)
            .block(panel_block(self.active_panel == Panel::Commits, "Commits"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.commit_state);
    }

    fn render_log_panel(&self, frame: &mut Frame, area: Rect) {
        let text = if self.log_entries.is_empty() {
            "No log messages yet.".to_string()
        } else {
            self.log_entries.join("\n")
        };

        let paragraph = Paragraph::new(text)
            .block(panel_block(self.active_panel == Panel::Log, "Log"))
            .scroll((self.log_scroll, 0));
        frame.render_widget(paragraph, area);
    }
}

fn default_list_state() -> ListState {
    let mut state = ListState::default();
    state.select(Some(0));
    state
}

fn sync_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0);
    state.select(Some(current.min(len.saturating_sub(1))));
}

fn cycle_selection(state: &mut ListState, len: usize, delta: i8) {
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0);
    let next = if delta > 0 {
        if current + 1 >= len {
            0
        } else {
            current + 1
        }
    } else if current == 0 {
        len.saturating_sub(1)
    } else {
        current - 1
    };

    state.select(Some(next));
}

fn apply_scroll(current: u16, delta: i16) -> u16 {
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as u16)
    }
}

fn panel_block(is_active: bool, title: &str) -> Block<'static> {
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title.to_string())
}

fn file_style(file: &StatusEntry) -> Style {
    match (file.is_staged, file.is_unstaged) {
        (true, true) => Style::default().fg(Color::LightCyan),
        (true, false) => Style::default().fg(Color::Green),
        (false, true) => Style::default().fg(Color::Yellow),
        (false, false) => Style::default(),
    }
}
