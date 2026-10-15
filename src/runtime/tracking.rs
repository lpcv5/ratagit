//! Runtime-only provenance; reducers and immutable domain snapshots stay unchanged.
use crate::flux::action::{Action, DomainAction, SystemAction};
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::latency::{Recorder, Trace};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug)]
pub(super) struct Tracked<T> {
    pub value: T,
    pub trace: Option<Trace>,
    pub(super) queued: Option<Instant>,
}

impl<T> Tracked<T> {
    pub fn new(value: T, trace: Option<Trace>) -> Self {
        let queued = trace.as_ref().map(|_| Instant::now());
        Self {
            value,
            trace,
            queued,
        }
    }

    pub fn dequeued(&self, stage: &'static str) {
        if let (Some(trace), Some(queued)) = (&self.trace, self.queued) {
            trace.event(stage, queued.elapsed());
        }
    }
}

impl<T> From<T> for Tracked<T> {
    fn from(value: T) -> Self {
        Self::new(value, None)
    }
}

#[derive(Clone)]
pub(super) struct ActionSender {
    tx: mpsc::UnboundedSender<Tracked<Action>>,
    recorder: Option<Arc<Recorder>>,
}

impl ActionSender {
    pub fn new(
        tx: mpsc::UnboundedSender<Tracked<Action>>,
        recorder: Option<Arc<Recorder>>,
    ) -> Self {
        Self { tx, recorder }
    }

    pub fn start(&self, origin: &'static str) -> Option<Trace> {
        self.recorder
            .as_ref()
            .map(|recorder| recorder.start(origin))
    }

    pub fn send(&self, action: Action) -> Result<(), mpsc::error::SendError<Action>> {
        let origin = match &action {
            Action::System(SystemAction::Tick | SystemAction::BackgroundCompleted(_)) => None,
            Action::System(SystemAction::AutoRefresh) => Some("auto_refresh"),
            Action::System(SystemAction::Resize { .. }) => Some("resize"),
            Action::Domain(DomainAction::Refresh) => Some("refresh"),
            _ => Some("input"),
        };
        let trace = origin.and_then(|origin| self.start(origin));
        self.send_traced(action, trace)
    }

    pub fn send_traced(
        &self,
        action: Action,
        trace: Option<Trace>,
    ) -> Result<(), mpsc::error::SendError<Action>> {
        if let Some(trace) = &trace {
            trace.event("action_queued", Duration::ZERO);
        }
        self.tx
            .send(Tracked::new(action, trace))
            .map_err(|error| mpsc::error::SendError(error.0.value))
    }
}

pub(super) struct PublishedSnapshot {
    state: AppStateSnapshotOwned,
    pub version: u64,
    trace: Option<Trace>,
    observed: AtomicBool,
}

impl PublishedSnapshot {
    pub fn new(state: AppStateSnapshotOwned, version: u64, trace: Option<Trace>) -> Self {
        if let Some(trace) = &trace {
            trace.snapshot_event("snapshot_published", Duration::ZERO, version);
        }
        Self {
            state,
            version,
            trace,
            observed: AtomicBool::new(false),
        }
    }

    pub fn finish(&self, stage: &'static str, duration: Duration) {
        if let Some(trace) = &self.trace {
            if !self.observed.swap(true, Ordering::Relaxed) {
                trace.snapshot_event(stage, duration, self.version);
            }
        }
    }
}

impl Deref for PublishedSnapshot {
    type Target = AppStateSnapshotOwned;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
