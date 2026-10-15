use crate::app::{AppRuntime, Command};
use crate::flux::action::Action;
use crate::flux::effects::{self, EffectRequest};
use crate::latency::Trace;
use std::collections::VecDeque;
use std::time::Instant;
use tokio::task::JoinSet;

const MAX_CONCURRENT_READS: usize = 6;

pub struct EffectCompletion {
    pub actions: Vec<Action>,
    pub elapsed_us: u64,
    pub(crate) trace: Option<Trace>,
}

struct PendingEffect {
    request: EffectRequest,
    trace: Option<Trace>,
    queued: Option<Instant>,
}

/// Local Git writes preserve arrival order. Reads and fetches do not wait for
/// that lane. Desktop effects also run outside reducer-owned state.
pub struct EffectExecutor {
    runtime: AppRuntime,
    writes: VecDeque<PendingEffect>,
    reads: VecDeque<PendingEffect>,
    jobs: JoinSet<(bool, EffectCompletion)>,
    active_reads: usize,
    active_write: bool,
}

impl EffectExecutor {
    pub fn new(runtime: AppRuntime) -> Self {
        Self {
            runtime,
            writes: VecDeque::new(),
            reads: VecDeque::new(),
            jobs: JoinSet::new(),
            active_reads: 0,
            active_write: false,
        }
    }

    pub fn enqueue(&mut self, request: EffectRequest) {
        self.enqueue_traced(request, None);
    }

    pub(crate) fn enqueue_traced(&mut self, request: EffectRequest, trace: Option<Trace>) {
        let pending = PendingEffect {
            queued: trace.as_ref().map(|_| Instant::now()),
            request,
            trace,
        };
        if pending.request.is_write() {
            self.writes.push_back(pending);
        } else {
            self.reads.push_back(pending);
        }
        self.start_ready();
    }

    fn start_ready(&mut self) {
        if !self.active_write {
            if let Some(request) = self.writes.pop_front() {
                self.active_write = true;
                self.start(request, true);
            }
        }
        while self.active_reads < MAX_CONCURRENT_READS {
            let Some(request) = self.reads.pop_front() else {
                break;
            };
            self.active_reads += 1;
            self.start(request, false);
        }
    }

    fn start(&mut self, pending: PendingEffect, write: bool) {
        let runtime = self.runtime.clone();
        self.jobs.spawn(async move {
            let started = Instant::now();
            if let (Some(trace), Some(queued)) = (&pending.trace, pending.queued) {
                trace.event("effect_started", queued.elapsed());
            }
            let actions =
                effects::run_traced(pending.request, &runtime, pending.trace.clone()).await;
            if let Some(trace) = &pending.trace {
                trace.event("effect_finished", started.elapsed());
            }
            (
                write,
                EffectCompletion {
                    actions,
                    elapsed_us: started.elapsed().as_micros() as u64,
                    trace: pending.trace,
                },
            )
        });
    }

    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    pub async fn next_completed(&mut self) -> EffectCompletion {
        let (write, completion) = self
            .jobs
            .join_next()
            .await
            .expect("completion requires an active job")
            .expect("effect tasks convert worker panics into completion events");
        if write {
            self.active_write = false;
        } else {
            self.active_reads -= 1;
        }
        self.start_ready();
        completion
    }
}

/// Relay FIFO commands at the original command-consumption boundary. Follow-up
/// actions go directly to the action queue; only I/O enters the effect executor.
/// Returning an action never reduces state recursively.
pub fn route_command(command: Command, executor: &mut EffectExecutor) -> Vec<Action> {
    match command {
        Command::RunEditor(_) => vec![Action::Domain(
            crate::flux::action::DomainAction::EditorFinished {
                result: Err(crate::git::GitError::Worker(
                    "editor requires the terminal UI".into(),
                )),
            },
        )],
        Command::None => Vec::new(),
        Command::Sync(action) => vec![Action::Domain(action)],
        Command::Effect(request) => {
            executor.enqueue(request);
            Vec::new()
        }
    }
}
