mod commands;
mod events;
pub mod git_ops;
mod handlers;
pub mod macros;
mod runtime;

pub use commands::{BackendCommand, CommandEnvelope, DiffTarget};
pub use events::{EventEnvelope, FrontendEvent};
pub use runtime::run_backend;
