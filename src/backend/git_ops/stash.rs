use anyhow::Result;

use super::repo::GitRepo;

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub id: String,
    pub message: String,
}

pub fn get_stashes(repo: &mut GitRepo) -> Result<Vec<StashEntry>> {
    let mut stashes = Vec::new();

    repo.repo.stash_foreach(|index, message, oid| {
        stashes.push(StashEntry {
            index,
            id: short_oid(&oid.to_string()),
            message: message.to_string(),
        });
        true
    })?;

    Ok(stashes)
}

fn short_oid(oid: &str) -> String {
    oid.chars().take(8).collect()
}
