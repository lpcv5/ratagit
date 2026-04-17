use std::collections::HashSet;

pub struct RequestTracker {
    pending: HashSet<u64>,
    latest_branch_commits: Option<u64>,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self {
            pending: HashSet::new(),
            latest_branch_commits: None,
        }
    }

    pub fn complete(&mut self, id: u64) -> bool {
        self.pending.remove(&id)
    }

    pub fn is_latest_branch_commits(&self, id: u64) -> bool {
        self.latest_branch_commits == Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_is_empty() {
        let tracker = RequestTracker::new();
        assert_eq!(tracker.pending.len(), 0);
        assert_eq!(tracker.latest_branch_commits, None);
    }

    #[test]
    fn test_complete_removes_pending() {
        let mut tracker = RequestTracker::new();
        tracker.pending.insert(1);
        tracker.pending.insert(2);

        // Complete request 1
        let removed = tracker.complete(1);
        assert!(removed);
        assert!(!tracker.pending.contains(&1));
        assert!(tracker.pending.contains(&2));

        // Complete non-existent request
        let removed = tracker.complete(999);
        assert!(!removed);
    }

    #[test]
    fn test_is_latest_branch_commits() {
        let mut tracker = RequestTracker::new();

        // Initially no latest
        assert!(!tracker.is_latest_branch_commits(1));

        // Set latest
        tracker.latest_branch_commits = Some(42);
        assert!(tracker.is_latest_branch_commits(42));
        assert!(!tracker.is_latest_branch_commits(43));
    }

    #[test]
    fn test_multiple_pending_requests() {
        let mut tracker = RequestTracker::new();

        // Add multiple pending requests
        tracker.pending.insert(1);
        tracker.pending.insert(2);
        tracker.pending.insert(3);

        assert_eq!(tracker.pending.len(), 3);

        // Complete them one by one
        tracker.complete(2);
        assert_eq!(tracker.pending.len(), 2);
        assert!(tracker.pending.contains(&1));
        assert!(tracker.pending.contains(&3));

        tracker.complete(1);
        tracker.complete(3);
        assert_eq!(tracker.pending.len(), 0);
    }
}
