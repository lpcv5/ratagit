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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_valid_repo() {
        // This test runs in the ratagit repo itself, which is a valid git repo
        let result = GitRepo::discover();
        assert!(result.is_ok());

        let repo = result.unwrap();
        assert!(repo.repo.workdir().is_some());
    }
}
