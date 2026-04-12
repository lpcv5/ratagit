use anyhow::{Context, Result};
use git2::Repository;

pub struct GitRepo {
    pub repo: Repository,
}

impl GitRepo {
    pub fn discover() -> Result<Self> {
        let repo =
            Repository::discover(".").context("Not a git repository (or any parent directory)")?;
        repo.workdir()
            .context("Repository has no working directory")?;

        Ok(Self { repo })
    }
}
