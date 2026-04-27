use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use ratagit_core::Command;

#[derive(Debug, Default)]
pub(crate) struct CommandScheduler {
    debounce_window: Duration,
    debounced: HashMap<&'static str, DebouncedCommand>,
}

#[derive(Debug, Clone)]
struct DebouncedCommand {
    due_at: Instant,
    command: Command,
}

impl CommandScheduler {
    pub(crate) fn set_debounce_window(&mut self, debounce_window: Duration) {
        self.debounce_window = debounce_window;
    }

    pub(crate) fn enqueue_at(
        &mut self,
        command: Command,
        queue: &mut VecDeque<Command>,
        now: Instant,
    ) {
        if self.debounce_window > Duration::ZERO
            && let Some(key) = command.debounce_key()
        {
            self.debounced.insert(
                key,
                DebouncedCommand {
                    due_at: now + self.debounce_window,
                    command,
                },
            );
            return;
        }
        enqueue_coalesced_command(queue, command);
    }

    pub(crate) fn flush_due_at(&mut self, now: Instant) -> VecDeque<Command> {
        if self.debounced.is_empty() {
            return VecDeque::new();
        }

        let due_keys = self
            .debounced
            .iter()
            .filter_map(|(key, pending)| (pending.due_at <= now).then_some(*key))
            .collect::<Vec<_>>();
        if due_keys.is_empty() {
            return VecDeque::new();
        }

        let mut queue = VecDeque::new();
        for key in due_keys {
            if let Some(pending) = self.debounced.remove(key) {
                enqueue_coalesced_command(&mut queue, pending.command);
            }
        }
        queue
    }
}

pub(crate) fn enqueue_coalesced_command(queue: &mut VecDeque<Command>, command: Command) {
    let search_start = queue
        .iter()
        .rposition(|command| command.is_mutating())
        .map_or(0, |index| index + 1);
    if let Some(target) = command.refresh_coalescing_key() {
        if !queue
            .iter()
            .skip(search_start)
            .any(|queued| queued.refresh_coalescing_key() == Some(target))
        {
            queue.push_back(command);
        }
        return;
    }

    if let Some(key) = command.debounce_key() {
        remove_queued_command_with_debounce_key(queue, search_start, key);
    }
    queue.push_back(command);
}

fn remove_queued_command_with_debounce_key(
    queue: &mut VecDeque<Command>,
    search_start: usize,
    key: &'static str,
) {
    if let Some(index) = queue
        .iter()
        .enumerate()
        .skip(search_start)
        .find_map(|(index, queued)| (queued.debounce_key() == Some(key)).then_some(index))
    {
        queue.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ratagit_core::{Command, FileDiffTarget};

    use ratagit_core::DetailsRequestId;

    use super::*;

    fn file_diff_target(path: &str) -> FileDiffTarget {
        FileDiffTarget {
            path: path.to_string(),
            untracked: false,
            is_directory_marker: path.ends_with('/'),
        }
    }

    fn details_diff(path: &str) -> Command {
        Command::RefreshFilesDetailsDiff {
            request_id: DetailsRequestId(0),
            targets: vec![file_diff_target(path)],
            truncated_from: None,
        }
    }

    #[test]
    fn debounce_window_defers_until_due_without_sleeping() {
        let now = Instant::now();
        let mut scheduler = CommandScheduler::default();
        scheduler.set_debounce_window(Duration::from_millis(50));
        let mut queue = VecDeque::new();

        scheduler.enqueue_at(details_diff("old.txt"), &mut queue, now);
        scheduler.enqueue_at(
            details_diff("latest.txt"),
            &mut queue,
            now + Duration::from_millis(10),
        );

        assert!(queue.is_empty());
        assert!(
            scheduler
                .flush_due_at(now + Duration::from_millis(59))
                .is_empty()
        );
        assert_eq!(
            scheduler
                .flush_due_at(now + Duration::from_millis(60))
                .into_iter()
                .collect::<Vec<_>>(),
            vec![details_diff("latest.txt")]
        );
    }

    #[test]
    fn zero_debounce_window_enqueues_immediately() {
        let mut scheduler = CommandScheduler::default();
        let mut queue = VecDeque::new();

        scheduler.enqueue_at(details_diff("current.txt"), &mut queue, Instant::now());

        assert_eq!(
            queue.into_iter().collect::<Vec<_>>(),
            vec![details_diff("current.txt")]
        );
    }

    #[test]
    fn command_coalescing_preserves_mutation_boundaries() {
        let mut queue = VecDeque::new();
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);
        enqueue_coalesced_command(
            &mut queue,
            Command::StageFiles {
                paths: vec!["a.txt".to_string()],
            },
        );
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);

        assert_eq!(
            queue.into_iter().collect::<Vec<_>>(),
            vec![
                Command::RefreshAll,
                Command::StageFiles {
                    paths: vec!["a.txt".to_string()]
                },
                Command::RefreshAll,
            ]
        );
    }

    #[test]
    fn command_coalescing_uses_core_refresh_keys() {
        let mut queue = VecDeque::new();
        enqueue_coalesced_command(&mut queue, Command::RefreshFiles);
        enqueue_coalesced_command(&mut queue, Command::RefreshBranches);
        enqueue_coalesced_command(&mut queue, Command::RefreshFiles);

        assert_eq!(
            queue.into_iter().collect::<Vec<_>>(),
            vec![Command::RefreshFiles, Command::RefreshBranches]
        );
    }

    #[test]
    fn command_coalescing_keeps_latest_details_after_last_mutation() {
        let mut queue = VecDeque::new();
        enqueue_coalesced_command(&mut queue, details_diff("old.txt"));
        enqueue_coalesced_command(
            &mut queue,
            Command::StageFiles {
                paths: vec!["a.txt".to_string()],
            },
        );
        enqueue_coalesced_command(&mut queue, details_diff("stale.txt"));
        enqueue_coalesced_command(&mut queue, details_diff("latest.txt"));

        assert_eq!(
            queue.into_iter().collect::<Vec<_>>(),
            vec![
                details_diff("old.txt"),
                Command::StageFiles {
                    paths: vec!["a.txt".to_string()]
                },
                details_diff("latest.txt"),
            ]
        );
    }
}
