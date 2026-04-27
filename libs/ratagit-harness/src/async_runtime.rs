use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::sync::Arc;
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

pub const DEFAULT_READ_WORKER_COUNT: usize = 4;

#[derive(Debug)]
pub struct AsyncRuntime<B: GitBackend + Send + 'static> {
    state: AppState,
    read_command_txs: Vec<Sender<WorkerMessage>>,
    write_command_tx: Sender<WorkerMessage>,
    result_rx: Receiver<WorkerResult>,
    workers: Vec<JoinHandle<()>>,
    terminal_size: TerminalSize,
    debounce_window: Duration,
    debounced: HashMap<&'static str, DebouncedCommand>,
    deferred_reads: VecDeque<Command>,
    next_read_worker: usize,
    repo_generation: u64,
    write_commands_in_flight: usize,
    _backend_type: PhantomData<B>,
}

#[derive(Debug, Clone)]
struct DebouncedCommand {
    due_at: Instant,
    command: Command,
}

#[derive(Debug, Clone)]
enum WorkerMessage {
    Run { generation: u64, command: Command },
    Stop,
}

#[derive(Debug)]
struct WorkerResult {
    generation: u64,
    kind: WorkerKind,
    result: GitResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerKind {
    Read,
    Write,
}

impl<B: GitBackend + Send + 'static> AsyncRuntime<B> {
    pub fn new<F>(state: AppState, backend_factory: F, terminal_size: TerminalSize) -> Self
    where
        F: Fn() -> B + Send + Sync + 'static,
    {
        let backend_factory = Arc::new(backend_factory);
        let (result_tx, result_rx) = mpsc::channel::<WorkerResult>();
        let mut read_command_txs = Vec::with_capacity(DEFAULT_READ_WORKER_COUNT);
        let mut workers = Vec::with_capacity(DEFAULT_READ_WORKER_COUNT + 1);

        for _ in 0..DEFAULT_READ_WORKER_COUNT {
            let (command_tx, worker) = spawn_worker(
                Arc::clone(&backend_factory),
                result_tx.clone(),
                WorkerKind::Read,
            );
            read_command_txs.push(command_tx);
            workers.push(worker);
        }

        let (write_command_tx, write_worker) =
            spawn_worker(backend_factory, result_tx, WorkerKind::Write);
        workers.push(write_worker);

        Self {
            state,
            read_command_txs,
            write_command_tx,
            result_rx,
            workers,
            terminal_size,
            debounce_window: Duration::default(),
            debounced: HashMap::new(),
            deferred_reads: VecDeque::new(),
            next_read_worker: 0,
            repo_generation: 0,
            write_commands_in_flight: 0,
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
        let mut queue = VecDeque::new();
        for command in initial {
            self.enqueue_command(command, &mut queue);
        }
        self.dispatch_queue(queue);
    }

    fn enqueue_command(&mut self, command: Command, queue: &mut VecDeque<Command>) {
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
        enqueue_coalesced_command(queue, command);
    }

    fn dispatch_queue(&mut self, mut queue: VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            self.dispatch_command(command);
        }
    }

    fn dispatch_command(&mut self, command: Command) {
        if command.is_mutating() {
            self.dispatch_write_command(command);
        } else if self.write_commands_in_flight == 0 {
            self.dispatch_read_command(command);
        } else {
            enqueue_coalesced_command(&mut self.deferred_reads, command);
        }
    }

    fn dispatch_read_command(&mut self, command: Command) {
        let Some(worker_count) = NonZeroWorkerCount::new(self.read_command_txs.len()) else {
            self.process_worker_failure("async git read worker pool is empty");
            return;
        };
        let worker_index = self.next_read_worker % worker_count.get();
        self.next_read_worker = (self.next_read_worker + 1) % worker_count.get();
        let message = WorkerMessage::Run {
            generation: self.repo_generation,
            command,
        };
        if self.read_command_txs[worker_index].send(message).is_err() {
            self.process_worker_failure("async git read worker stopped");
        }
    }

    fn dispatch_write_command(&mut self, command: Command) {
        self.repo_generation = self.repo_generation.wrapping_add(1);
        self.write_commands_in_flight = self.write_commands_in_flight.saturating_add(1);
        let message = WorkerMessage::Run {
            generation: self.repo_generation,
            command,
        };
        if self.write_command_tx.send(message).is_err() {
            self.write_commands_in_flight = self.write_commands_in_flight.saturating_sub(1);
            self.process_worker_failure("async git write worker stopped");
            self.flush_deferred_reads_if_unblocked();
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
        if due_keys.is_empty() {
            return;
        }

        let mut queue = VecDeque::new();
        for key in due_keys {
            if let Some(pending) = self.debounced.remove(key) {
                enqueue_coalesced_command(&mut queue, pending.command);
            }
        }
        self.dispatch_queue(queue);
    }

    fn drain_results(&mut self) {
        loop {
            match self.result_rx.try_recv() {
                Ok(worker_result) => self.process_worker_result(worker_result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.process_worker_failure("async git worker pool disconnected");
                    break;
                }
            }
        }
    }

    fn process_worker_result(&mut self, worker_result: WorkerResult) {
        match worker_result.kind {
            WorkerKind::Read => {
                if worker_result.generation == self.repo_generation {
                    self.process_git_result(worker_result.result);
                }
            }
            WorkerKind::Write => {
                self.write_commands_in_flight = self.write_commands_in_flight.saturating_sub(1);
                self.process_git_result(worker_result.result);
                self.flush_deferred_reads_if_unblocked();
            }
        }
    }

    fn process_git_result(&mut self, git_result: GitResult) {
        let follow_up = update(&mut self.state, Action::GitResult(git_result));
        self.process_commands(follow_up);
    }

    fn process_worker_failure(&mut self, error: &str) {
        self.process_git_result(GitResult::RefreshFailed {
            error: error.to_string(),
        });
    }

    fn flush_deferred_reads_if_unblocked(&mut self) {
        if self.write_commands_in_flight > 0 || self.deferred_reads.is_empty() {
            return;
        }

        let queue = std::mem::take(&mut self.deferred_reads);
        self.dispatch_queue(queue);
    }
}

impl<B: GitBackend + Send + 'static> Drop for AsyncRuntime<B> {
    fn drop(&mut self) {
        for command_tx in &self.read_command_txs {
            let _ = command_tx.send(WorkerMessage::Stop);
        }
        let _ = self.write_command_tx.send(WorkerMessage::Stop);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NonZeroWorkerCount(usize);

impl NonZeroWorkerCount {
    fn new(value: usize) -> Option<Self> {
        (value > 0).then_some(Self(value))
    }

    fn get(self) -> usize {
        self.0
    }
}

fn spawn_worker<B, F>(
    backend_factory: Arc<F>,
    result_tx: Sender<WorkerResult>,
    kind: WorkerKind,
) -> (Sender<WorkerMessage>, JoinHandle<()>)
where
    B: GitBackend + Send + 'static,
    F: Fn() -> B + Send + Sync + 'static,
{
    let (command_tx, command_rx) = mpsc::channel::<WorkerMessage>();
    let mut backend = backend_factory();
    let worker = thread::spawn(move || {
        run_worker(&mut backend, command_rx, result_tx, kind);
    });
    (command_tx, worker)
}

fn run_worker(
    backend: &mut dyn GitBackend,
    command_rx: Receiver<WorkerMessage>,
    result_tx: Sender<WorkerResult>,
    kind: WorkerKind,
) {
    while let Ok(message) = command_rx.recv() {
        match message {
            WorkerMessage::Run {
                generation,
                command,
            } => {
                let result = execute_command(backend, command);
                if result_tx
                    .send(WorkerResult {
                        generation,
                        kind,
                        result,
                    })
                    .is_err()
                {
                    return;
                }
            }
            WorkerMessage::Stop => break,
        }
    }
}

#[cfg(test)]
mod tests;
