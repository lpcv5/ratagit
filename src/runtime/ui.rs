//! Terminal input and immutable snapshot rendering.
use super::telemetry::{update_backlog, update_max, PerfCounters, PerfLog, ENABLE_DEBUG_LOG};
use crate::flux;
use crate::flux::action::{Action, SystemAction};
use crate::flux::snapshot::AppStateSnapshotOwned;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;
use std::sync::{atomic::Ordering, Arc};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};
use tracing::info;

pub(super) async fn ui_loop(
    terminal: &mut DefaultTerminal,
    perf: Arc<PerfCounters>,
    action_tx: mpsc::UnboundedSender<Action>,
    state_version_rx: &mut watch::Receiver<u64>,
    mut snapshot_rx: watch::Receiver<Arc<AppStateSnapshotOwned>>,
    shutdown_tx: broadcast::Sender<()>,
) -> Result<()> {
    const MAX_EVENTS_PER_FRAME: usize = 64;
    const DIFF_RELOAD_DEBOUNCE: Duration = Duration::from_millis(180);
    const SLOW_DRAW_US: u64 = 8_000;
    let mut perf_log = if ENABLE_DEBUG_LOG {
        PerfLog::new().ok()
    } else {
        None
    };
    if let Some(log) = perf_log.as_mut() {
        log.write_line("ratagit debug perf started");
    }

    loop {
        // Check exit status at loop start
        {
            let snapshot = snapshot_rx.borrow();
            if !snapshot.running {
                info!("ui_loop: detected running=false, exiting");
                let _ = shutdown_tx.send(());
                return Ok(());
            }
        }

        let mut processed_input_event = false;
        if state_version_rx.has_changed().unwrap_or(false) {
            let _ = state_version_rx.borrow_and_update();
        }

        if snapshot_rx.has_changed().unwrap_or(false) {
            let draw_started = Instant::now();
            terminal.draw(|f| {
                let snapshot = snapshot_rx.borrow_and_update();
                let view = snapshot.as_snapshot();
                crate::ui::layout::render_layout(f, &view);
            })?;
            let draw_us = draw_started.elapsed().as_micros() as u64;
            perf.ui_draws.fetch_add(1, Ordering::Relaxed);
            perf.ui_draw_us.fetch_add(draw_us, Ordering::Relaxed);
            update_max(&perf.ui_draw_max_us, draw_us);
            if let Some(log) = perf_log.as_mut() {
                if draw_us >= SLOW_DRAW_US {
                    log.write_line(&format!("slow_draw_us={}", draw_us));
                }
            }
        }

        if event::poll(Duration::from_millis(0))? {
            for _ in 0..MAX_EVENTS_PER_FRAME {
                match event::read()? {
                    Event::Key(key) => {
                        processed_input_event = true;
                        perf.ui_events.fetch_add(1, Ordering::Relaxed);
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                            let key_started = Instant::now();
                            let snapshot = snapshot_rx.borrow();
                            let view = snapshot.as_snapshot();
                            let mapped_actions = flux::input_mapper::map_key_to_actions(key, &view);
                            for action in mapped_actions {
                                let _ = action_tx.send(action);
                                perf.ui_messages.fetch_add(1, Ordering::Relaxed);
                                perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
                                update_backlog(
                                    &perf.action_enqueued,
                                    &perf.action_dequeued,
                                    &perf.action_backlog_max,
                                );
                            }
                            let key_us = key_started.elapsed().as_micros() as u64;
                            perf.ui_key_handle_us.fetch_add(key_us, Ordering::Relaxed);
                            update_max(&perf.ui_key_handle_max_us, key_us);
                        }
                    }
                    Event::Resize(width, height) => {
                        processed_input_event = true;
                        perf.ui_events.fetch_add(1, Ordering::Relaxed);
                        let _ =
                            action_tx.send(Action::System(SystemAction::Resize { width, height }));
                        perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
                        update_backlog(
                            &perf.action_enqueued,
                            &perf.action_dequeued,
                            &perf.action_backlog_max,
                        );
                    }
                    _ => {}
                }

                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        let should_tick = {
            let snapshot = snapshot_rx.borrow();
            snapshot.should_tick(DIFF_RELOAD_DEBOUNCE)
        };
        if should_tick {
            perf.ui_ticks_sent.fetch_add(1, Ordering::Relaxed);
            let _ = action_tx.send(Action::System(SystemAction::Tick));
            perf.action_enqueued.fetch_add(1, Ordering::Relaxed);
            update_backlog(
                &perf.action_enqueued,
                &perf.action_dequeued,
                &perf.action_backlog_max,
            );
        }

        if let Some(log) = perf_log.as_mut() {
            log.record(&perf);
        }

        let frame_sleep = if processed_input_event {
            Duration::from_millis(1)
        } else {
            Duration::from_millis(16)
        };
        tokio::time::sleep(frame_sleep).await;
    }
}
