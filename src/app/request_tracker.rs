use std::collections::HashSet;

pub struct RequestTracker {
    pending: HashSet<u64>,
    latest_diff: Option<u64>,
    latest_branch_graph: Option<u64>,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self {
            pending: HashSet::new(),
            latest_diff: None,
            latest_branch_graph: None,
        }
    }

    pub fn track(&mut self, id: u64) {
        if id != 0 {
            self.pending.insert(id);
        }
    }

    pub fn complete(&mut self, id: u64) -> bool {
        self.pending.remove(&id)
    }

    pub fn set_latest_diff(&mut self, id: u64) {
        if id == 0 {
            return;
        }
        if let Some(prev) = self.latest_diff.replace(id) {
            self.pending.remove(&prev);
        }
        self.pending.insert(id);
    }

    pub fn set_latest_branch_graph(&mut self, id: u64) {
        if id == 0 {
            return;
        }
        if let Some(prev) = self.latest_branch_graph.replace(id) {
            self.pending.remove(&prev);
        }
        self.pending.insert(id);
    }
}
