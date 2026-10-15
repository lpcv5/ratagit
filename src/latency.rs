//! Opt-in causal timing, independent of application state and Git data.
use serde::Serialize;
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

thread_local! {
    static CURRENT: RefCell<Option<Trace>> = const { RefCell::new(None) };
}

pub(crate) struct Recorder {
    writer: Mutex<Box<dyn Write + Send>>,
    next_id: AtomicU64,
}

#[derive(Serialize)]
struct Event {
    schema: u8,
    request_id: u64,
    span_id: u64,
    parent_span_id: Option<u64>,
    origin: &'static str,
    operation: &'static str,
    generation: Option<u64>,
    stage: &'static str,
    elapsed_us: u64,
    span_us: u64,
    duration_us: u64,
    state_version: Option<u64>,
    label: Option<&'static str>,
    lane: Option<usize>,
}

impl Recorder {
    pub(crate) fn open(path: impl AsRef<std::path::Path>) -> io::Result<Arc<Self>> {
        Ok(Self::with_writer(File::create(path)?))
    }

    pub(crate) fn with_writer(writer: impl Write + Send + 'static) -> Arc<Self> {
        Arc::new(Self {
            writer: Mutex::new(Box::new(writer)),
            next_id: AtomicU64::new(1),
        })
    }

    pub(crate) fn start(self: &Arc<Self>, origin: &'static str) -> Trace {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();
        let trace = Trace(Arc::new(Span {
            root: Arc::new(Root {
                recorder: self.clone(),
                id,
                origin,
                started: now,
            }),
            span_id: id,
            parent_span_id: None,
            operation: origin,
            generation: None,
            started: now,
            frame_reported: AtomicBool::new(false),
        }));
        trace.event("request_started", Duration::ZERO);
        trace
    }
}

struct Root {
    recorder: Arc<Recorder>,
    id: u64,
    origin: &'static str,
    started: Instant,
}

#[derive(Clone)]
pub(crate) struct Trace(Arc<Span>);

struct Span {
    root: Arc<Root>,
    span_id: u64,
    parent_span_id: Option<u64>,
    operation: &'static str,
    generation: Option<u64>,
    started: Instant,
    frame_reported: AtomicBool,
}

impl std::fmt::Debug for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Trace")
            .field("request_id", &self.0.root.id)
            .field("span_id", &self.0.span_id)
            .field("operation", &self.0.operation)
            .finish()
    }
}

impl Trace {
    pub(crate) fn child(&self, operation: &'static str, generation: Option<u64>) -> Self {
        Self(Arc::new(Span {
            root: self.0.root.clone(),
            span_id: self.0.root.recorder.next_id.fetch_add(1, Ordering::Relaxed),
            parent_span_id: Some(self.0.span_id),
            operation,
            generation,
            started: Instant::now(),
            frame_reported: AtomicBool::new(false),
        }))
    }

    pub(crate) fn current() -> Option<Self> {
        CURRENT.with_borrow(Clone::clone)
    }

    /// Only synchronous work may run inside this scope; never hold it across await.
    pub(crate) fn scope<T>(trace: Option<Self>, work: impl FnOnce() -> T) -> T {
        struct Restore(Option<Trace>);
        impl Drop for Restore {
            fn drop(&mut self) {
                CURRENT.with_borrow_mut(|current| *current = self.0.take());
            }
        }
        let _restore = Restore(CURRENT.replace(trace));
        work()
    }

    pub(crate) fn event(&self, stage: &'static str, duration: Duration) {
        self.worker_event(stage, duration, None, None);
    }

    pub(crate) fn worker_event(
        &self,
        stage: &'static str,
        duration: Duration,
        label: Option<&'static str>,
        lane: Option<usize>,
    ) {
        self.write_event(stage, duration, label, lane, None);
    }

    pub(crate) fn snapshot_event(&self, stage: &'static str, duration: Duration, version: u64) {
        if stage == "frame_drawn" && self.0.frame_reported.swap(true, Ordering::Relaxed) {
            return;
        }
        self.write_event(stage, duration, None, None, Some(version));
    }

    fn write_event(
        &self,
        stage: &'static str,
        duration: Duration,
        label: Option<&'static str>,
        lane: Option<usize>,
        state_version: Option<u64>,
    ) {
        let now = Instant::now();
        let event = Event {
            schema: 1,
            request_id: self.0.root.id,
            span_id: self.0.span_id,
            parent_span_id: self.0.parent_span_id,
            origin: self.0.root.origin,
            operation: self.0.operation,
            generation: self.0.generation,
            stage,
            elapsed_us: micros(now.saturating_duration_since(self.0.root.started)),
            span_us: micros(now.saturating_duration_since(self.0.started)),
            duration_us: micros(duration),
            state_version,
            label,
            lane,
        };
        // Diagnostics must never fail an operation or write into the terminal.
        if let Ok(mut writer) = self.0.root.recorder.writer.lock() {
            let _ = serde_json::to_writer(&mut *writer, &event);
            let _ = writer.write_all(b"\n");
        }
    }
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros().min(u64::MAX as u128) as u64
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[derive(Clone, Default)]
    pub(crate) struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Capture {
        pub(crate) fn events(&self) -> Vec<serde_json::Value> {
            String::from_utf8(self.0.lock().unwrap().clone())
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        }
    }

    #[test]
    fn test_trace_preserves_causality_and_restores_scope_on_unwind() {
        let capture = Capture::default();
        let recorder = Recorder::with_writer(capture.clone());
        let root = recorder.start("input");
        let child = root.child("diff", Some(42));
        assert!(Trace::current().is_none());
        Trace::scope(Some(root), || {
            let _ = std::panic::catch_unwind(|| {
                Trace::scope(Some(child), || panic!("controlled panic"));
            });
            Trace::current().unwrap().event("restored", Duration::ZERO);
        });
        assert!(Trace::current().is_none());
        let events = capture.events();
        assert_eq!(events[0]["request_id"], events[1]["request_id"]);
        assert_eq!(events[1]["operation"], "input");
        assert_eq!(events[1]["stage"], "restored");
    }
}
