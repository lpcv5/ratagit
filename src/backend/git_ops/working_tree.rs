use anyhow::Result;

use super::repo::GitRepo;

pub fn stage_file(repo: &GitRepo, file_path: &str) -> Result<()> {
    let mut index = repo.repo.index()?;
    index.add_path(file_path.as_ref())?;
    Ok(index.write()?)
}

pub fn unstage_file(repo: &GitRepo, file_path: &str) -> Result<()> {
    // 从 index 中移除文件
    let mut index = repo.repo.index()?;
    index.remove_path(file_path.as_ref())?;
    Ok(index.write()?)
}
