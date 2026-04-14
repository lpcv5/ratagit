mod fixtures;

use fixtures::TestRepo;

#[test]
fn test_fixture_smoke_test() {
    // Test that TestRepo can create a valid Git repository
    let test_repo = TestRepo::new();
    assert!(test_repo.path.exists());
    assert!(test_repo.path.join(".git").exists());

    // Test file creation
    test_repo.create_file("test.txt", "hello");
    assert!(test_repo.path.join("test.txt").exists());

    // Test commit
    test_repo.stage_file("test.txt");
    test_repo.commit("Test commit");

    // Verify we have at least 2 commits (initial + test)
    let mut revwalk = test_repo.repo.revwalk().unwrap();
    revwalk.push_head().unwrap();
    assert!(revwalk.count() >= 2);
}
