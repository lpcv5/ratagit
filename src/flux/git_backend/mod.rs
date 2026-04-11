pub mod detail;
pub mod stash;

pub mod branches {
    #[allow(unused_imports)]
    pub use crate::flux::branch_backend::*;
}

pub mod commits {
    #[allow(unused_imports)]
    pub use crate::flux::commits_backend::*;
}

pub mod files {
    #[allow(unused_imports)]
    pub use crate::flux::files_backend::*;
}

#[derive(Debug, Clone)]
pub enum GitBackendCommand {
    Stash(stash::StashBackendCommand),
}
