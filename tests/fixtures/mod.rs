use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test fixture for creating temporary Git repositories
#[allow(dead_code)]
pub struct TestRepo {
    pub temp_dir: TempDir,
    pub repo: git2::Repository,
    pub path: PathBuf,
}

#[allow(dead_code)]
impl TestRepo {
    /// Create a new empty repository with an initial commit
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let path = temp_dir.path().to_path_buf();

        // Initialize git repo
        let repo = git2::Repository::init(&path).expect("Failed to init repo");

        // Configure user for commits
        {
            let mut config = repo.config().expect("Failed to get config");
            config
                .set_str("user.name", "Test User")
                .expect("Failed to set user.name");
            config
                .set_str("user.email", "test@example.com")
                .expect("Failed to set user.email");
        }

        // Create initial commit to avoid empty repo issues
        {
            let sig = repo.signature().expect("Failed to create signature");
            let tree_id = {
                let mut index = repo.index().expect("Failed to get index");
                index.write_tree().expect("Failed to write tree")
            };
            let tree = repo.find_tree(tree_id).expect("Failed to find tree");
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .expect("Failed to create initial commit");
        }

        Self {
            temp_dir,
            repo,
            path,
        }
    }

    /// Create a file in the working directory
    pub fn create_file(&self, path: &str, content: &str) {
        let file_path = self.path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        fs::write(&file_path, content).expect("Failed to write file");
    }

    /// Stage a file
    pub fn stage_file(&self, path: &str) {
        let mut index = self.repo.index().expect("Failed to get index");
        index
            .add_path(Path::new(path))
            .expect("Failed to stage file");
        index.write().expect("Failed to write index");
    }

    /// Create a commit with the current staged changes
    pub fn commit(&self, message: &str) {
        let sig = self.repo.signature().expect("Failed to create signature");
        let mut index = self.repo.index().expect("Failed to get index");
        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = self.repo.find_tree(tree_id).expect("Failed to find tree");

        let parent_commit = self
            .repo
            .head()
            .ok()
            .and_then(|head: git2::Reference| head.peel_to_commit().ok());

        let parents = if let Some(ref parent) = parent_commit {
            vec![parent]
        } else {
            vec![]
        };

        self.repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .expect("Failed to create commit");
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) {
        let head = self.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        self.repo
            .branch(name, &commit, false)
            .expect("Failed to create branch");
    }

    /// Checkout a branch
    pub fn checkout(&self, branch_name: &str) {
        let obj = self
            .repo
            .revparse_single(&format!("refs/heads/{}", branch_name))
            .expect("Failed to find branch");
        self.repo
            .checkout_tree(&obj, None)
            .expect("Failed to checkout tree");
        self.repo
            .set_head(&format!("refs/heads/{}", branch_name))
            .expect("Failed to set HEAD");
    }

    /// Builder: create repo with an uncommitted file
    pub fn with_file(path: &str, content: &str) -> Self {
        let test_repo = Self::new();
        test_repo.create_file(path, content);
        test_repo
    }

    /// Builder: create repo with a staged file
    pub fn with_staged_file(path: &str, content: &str) -> Self {
        let test_repo = Self::new();
        test_repo.create_file(path, content);
        test_repo.stage_file(path);
        test_repo
    }

    /// Builder: create repo with a branch
    pub fn with_branch(name: &str) -> Self {
        let test_repo = Self::new();
        test_repo.create_branch(name);
        test_repo
    }

    /// Builder: create repo with multiple commits
    pub fn with_commits(count: usize) -> Self {
        let test_repo = Self::new();
        for i in 1..=count {
            let filename = format!("file{}.txt", i);
            test_repo.create_file(&filename, &format!("content {}", i));
            test_repo.stage_file(&filename);
            test_repo.commit(&format!("Commit {}", i));
        }
        test_repo
    }
}
