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
