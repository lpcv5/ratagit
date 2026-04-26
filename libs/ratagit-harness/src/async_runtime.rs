use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ratagit_core::{Action, AppState, Command, GitResult, UiAction, update};
use ratagit_git::{GitBackend, execute_command};
use ratagit_ui::{
    RenderedFrame, TerminalBuffer, TerminalSize, render, render_terminal_buffer,
    render_terminal_text,
};

use crate::enqueue_coalesced_command;

#[derive(Debug)]
pub struct AsyncRuntime<B: GitBackend + Send + 'static> {
    state: AppState,
    command_tx: Sender<WorkerMessage>,
    result_rx: Receiver<GitResult>,
    worker: Option<JoinHandle<()>>,
    terminal_size: TerminalSize,
    debounce_window: Duration,
    debounced: HashMap<&'static str, DebouncedCommand>,
    _backend_type: PhantomData<B>,
}

#[derive(Debug, Clone)]
struct DebouncedCommand {
    due_at: Instant,
    command: Command,
}

#[derive(Debug, Clone)]
enum WorkerMessage {
    Run(Command),
    Stop,
}

impl<B: GitBackend + Send + 'static> AsyncRuntime<B> {
    pub fn new(state: AppState, backend: B, terminal_size: TerminalSize) -> Self {
        let (command_tx, command_rx) = mpsc::channel::<WorkerMessage>();
        let (result_tx, result_rx) = mpsc::channel::<GitResult>();
        let worker = thread::spawn(move || {
            let mut backend = backend;
            while let Ok(message) = command_rx.recv() {
                match message {
                    WorkerMessage::Run(command) => {
                        let mut queue = VecDeque::new();
                        enqueue_coalesced_command(&mut queue, command);
                        loop {
                            match command_rx.try_recv() {
                                Ok(WorkerMessage::Run(command)) => {
                                    enqueue_coalesced_command(&mut queue, command);
                                }
                                Ok(WorkerMessage::Stop) => return,
                                Err(mpsc::TryRecvError::Empty) => break,
                                Err(mpsc::TryRecvError::Disconnected) => return,
                            }
                        }
                        while let Some(command) = queue.pop_front() {
                            if result_tx
                                .send(execute_command(&mut backend, command))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    WorkerMessage::Stop => break,
                }
            }
        });
        Self {
            state,
            command_tx,
            result_rx,
            worker: Some(worker),
            terminal_size,
            debounce_window: Duration::default(),
            debounced: HashMap::new(),
            _backend_type: PhantomData,
        }
    }

    pub fn with_debounce_window(mut self, debounce_window: Duration) -> Self {
        self.debounce_window = debounce_window;
        self
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn dispatch_ui(&mut self, action: UiAction) {
        let initial_commands = update(&mut self.state, Action::Ui(action));
        self.process_commands(initial_commands);
    }

    pub fn tick(&mut self) {
        self.drain_results();
        self.flush_due_debounced();
        self.drain_results();
    }

    pub fn render(&self) -> RenderedFrame {
        render(&self.state, self.terminal_size)
    }

    pub fn render_terminal_text(&self) -> String {
        render_terminal_text(&self.state, self.terminal_size)
    }

    pub fn render_terminal_buffer(&self) -> TerminalBuffer {
        render_terminal_buffer(&self.state, self.terminal_size)
    }

    fn process_commands(&mut self, initial: Vec<Command>) {
        for command in initial {
            self.enqueue_command(command);
        }
    }

    fn enqueue_command(&mut self, command: Command) {
        if self.debounce_window > Duration::ZERO
            && let Some(key) = command.debounce_key()
        {
            self.debounced.insert(
                key,
                DebouncedCommand {
                    due_at: Instant::now() + self.debounce_window,
                    command,
                },
            );
            return;
        }
        self.send_command(command);
    }

    fn send_command(&mut self, command: Command) {
        if self.command_tx.send(WorkerMessage::Run(command)).is_err() {
            let follow_up = update(
                &mut self.state,
                Action::GitResult(GitResult::RefreshFailed {
                    error: "async git worker stopped".to_string(),
                }),
            );
            self.process_commands(follow_up);
        }
    }

    fn flush_due_debounced(&mut self) {
        if self.debounced.is_empty() {
            return;
        }

        let now = Instant::now();
        let due_keys = self
            .debounced
            .iter()
            .filter_map(|(key, pending)| (pending.due_at <= now).then_some(*key))
            .collect::<Vec<_>>();
        for key in due_keys {
            if let Some(pending) = self.debounced.remove(key) {
                self.send_command(pending.command);
            }
        }
    }

    fn drain_results(&mut self) {
        loop {
            match self.result_rx.try_recv() {
                Ok(git_result) => {
                    let follow_up = update(&mut self.state, Action::GitResult(git_result));
                    self.process_commands(follow_up);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    let follow_up = update(
                        &mut self.state,
                        Action::GitResult(GitResult::RefreshFailed {
                            error: "async git worker disconnected".to_string(),
                        }),
                    );
                    self.process_commands(follow_up);
                    break;
                }
            }
        }
    }
}

impl<B: GitBackend + Send + 'static> Drop for AsyncRuntime<B> {
    fn drop(&mut self) {
        let _ = self.command_tx.send(WorkerMessage::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
