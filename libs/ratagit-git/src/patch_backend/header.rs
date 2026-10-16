//! Retain native metadata without allocating an unbounded textual patch body.
use crate::GitError;

pub(super) fn native_header(raw: &mut git2::Patch<'_>) -> Result<Vec<u8>, GitError> {
    let mut bytes = Vec::new();
    let mut stopped_at_body = false;
    let result = raw.print(&mut |_, _, line| {
        if matches!(line.origin(), 'F' | 'B') {
            bytes.extend_from_slice(line.content());
            true
        } else {
            stopped_at_body = true;
            false
        }
    });
    // Stopping at the first hunk deliberately cancels the print callback.
    if !stopped_at_body {
        result?;
    }
    Ok(bytes)
}
