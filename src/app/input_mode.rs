use crate::app::{App, CommitFieldFocus, InputMode, Message};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

impl App {
    pub(super) fn handle_input_key(&mut self, key: KeyEvent) -> Option<Message> {
        let mode = self.input_mode?;

        match key.code {
            KeyCode::Esc => {
                self.cancel_input();
                None
            }
            KeyCode::Tab => match mode {
                InputMode::CommitEditor => {
                    self.commit_focus = match self.commit_focus {
                        CommitFieldFocus::Message => CommitFieldFocus::Description,
                        CommitFieldFocus::Description => CommitFieldFocus::Message,
                    };
                    None
                }
                InputMode::CreateBranch | InputMode::StashEditor => None,
            },
            KeyCode::Enter => match mode {
                InputMode::CommitEditor => match self.commit_focus {
                    CommitFieldFocus::Message => {
                        let title = self.commit_message_buffer.trim().to_string();
                        if title.is_empty() {
                            self.push_log("Empty commit message ignored", false);
                            return None;
                        }
                        let description = self.commit_description_buffer.trim_end();
                        let value = if description.is_empty() {
                            title
                        } else {
                            format!("{}\n\n{}", title, description)
                        };
                        self.input_mode = None;
                        self.commit_message_buffer.clear();
                        self.commit_description_buffer.clear();
                        self.commit_focus = CommitFieldFocus::Message;
                        Some(Message::Commit(value))
                    }
                    CommitFieldFocus::Description => {
                        self.commit_description_buffer.push('\n');
                        None
                    }
                },
                InputMode::CreateBranch => {
                    let value = self.input_buffer.trim().to_string();
                    self.input_mode = None;
                    self.input_buffer.clear();

                    if value.is_empty() {
                        self.push_log("Empty input ignored", false);
                        return None;
                    }
                    Some(Message::CreateBranch(value))
                }
                InputMode::StashEditor => {
                    let value = self.stash_message_buffer.trim().to_string();
                    let paths = self.stash_targets.clone();
                    self.input_mode = None;
                    self.stash_message_buffer.clear();
                    self.stash_targets.clear();

                    if value.is_empty() {
                        self.push_log("Empty stash title ignored", false);
                        return None;
                    }
                    if paths.is_empty() {
                        self.push_log("stash blocked: no selected items", false);
                        return None;
                    }
                    Some(Message::StashPush {
                        message: value,
                        paths,
                    })
                }
            },
            KeyCode::Backspace => match mode {
                InputMode::CommitEditor => {
                    match self.commit_focus {
                        CommitFieldFocus::Message => {
                            self.commit_message_buffer.pop();
                        }
                        CommitFieldFocus::Description => {
                            self.commit_description_buffer.pop();
                        }
                    }
                    None
                }
                InputMode::CreateBranch => {
                    self.input_buffer.pop();
                    None
                }
                InputMode::StashEditor => {
                    self.stash_message_buffer.pop();
                    None
                }
            },
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return None;
                }
                match mode {
                    InputMode::CommitEditor => {
                        match self.commit_focus {
                            CommitFieldFocus::Message => self.commit_message_buffer.push(c),
                            CommitFieldFocus::Description => self.commit_description_buffer.push(c),
                        }
                        None
                    }
                    InputMode::CreateBranch => {
                        self.input_buffer.push(c);
                        None
                    }
                    InputMode::StashEditor => {
                        self.stash_message_buffer.push(c);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    pub fn start_commit_editor(&mut self) {
        self.input_mode = Some(InputMode::CommitEditor);
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
    }

    pub fn start_commit_editor_guarded(&mut self) -> bool {
        if self.status.staged.is_empty() {
            self.push_log("nothing staged to commit", false);
            return false;
        }
        self.start_commit_editor();
        true
    }

    pub fn start_branch_create_input(&mut self) {
        self.input_mode = Some(InputMode::CreateBranch);
        self.input_buffer.clear();
    }

    pub fn start_stash_editor(&mut self, targets: Vec<PathBuf>) {
        self.input_mode = Some(InputMode::StashEditor);
        self.stash_targets = targets;
        self.stash_message_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = None;
        self.input_buffer.clear();
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
        self.stash_message_buffer.clear();
        self.stash_targets.clear();
    }
}
