mod fixtures;

use fixtures::TestRepo;
use ratagit::backend::{run_backend, BackendCommand, CommandEnvelope, EventEnvelope, FrontendEvent};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Helper to create backend channels and spawn backend task
async fn setup_backend(repo_path: &str) -> (mpsc::Sender<CommandEnvelope>, mpsc::Receiver<EventEnvelope>) {
    let (cmd_tx, cmd_rx) = mpsc::channel(100);
    let (event_tx, event_rx) = mpsc::channel(100);

    let repo_path = repo_path.to_string();
    tokio::spawn(async move {
        // Change to the repo directory before running backend
        std::env::set_current_dir(&repo_path).ok();
        run_backend(cmd_rx, event_tx).await;
    });

    (cmd_tx, event_rx)
}

/// Helper to send command and receive response with specific request_id
async fn send_and_receive(
    cmd_tx: &mpsc::Sender<CommandEnvelope>,
    event_rx: &mut mpsc::Receiver<EventEnvelope>,
    request_id: u64,
    command: BackendCommand,
) -> Option<EventEnvelope> {
    let envelope = CommandEnvelope::new(request_id, command);
    cmd_tx.send(envelope).await.ok()?;

    // Wait up to 2 seconds for response with matching request_id
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        match timeout(deadline.duration_since(tokio::time::Instant::now()), event_rx.recv()).await {
            Ok(Some(envelope)) => {
                // Return the first event with matching request_id
                if envelope.request_id == Some(request_id) {
                    return Some(envelope);
                }
                // Skip events with None or different request_id (auto-refresh events)
            }
            _ => return None,
        }
    }
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_refresh_status_command() {
    let test_repo = TestRepo::new();
    test_repo.create_file("test.txt", "hello");
    test_repo.stage_file("test.txt");

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    let response = send_and_receive(&cmd_tx, &mut event_rx, 1, BackendCommand::RefreshStatus)
        .await
        .expect("Should receive response");

    assert_eq!(response.request_id, Some(1));
    match response.event {
        FrontendEvent::FilesUpdated { files } => {
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].path, "test.txt");
        }
        _ => panic!("Expected FilesUpdated event"),
    }

    // Send Quit command
    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_refresh_branches_command() {
    let test_repo = TestRepo::with_branch("feature");

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    let response = send_and_receive(&cmd_tx, &mut event_rx, 2, BackendCommand::RefreshBranches)
        .await
        .expect("Should receive response");

    assert_eq!(response.request_id, Some(2));
    match response.event {
        FrontendEvent::BranchesUpdated { branches } => {
            assert!(branches.len() >= 2); // default branch + feature
            // Check that feature branch exists
            assert!(branches.iter().any(|b| b.name == "feature"));
            // Check that we have at least one other branch (master or main)
            assert!(branches.iter().any(|b| b.name == "master" || b.name == "main"));
        }
        _ => panic!("Expected BranchesUpdated event"),
    }

    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_refresh_commits_command() {
    let test_repo = TestRepo::with_commits(5);

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    let response = send_and_receive(&cmd_tx, &mut event_rx, 3, BackendCommand::RefreshCommits { limit: 100 })
        .await
        .expect("Should receive response");

    assert_eq!(response.request_id, Some(3));
    match response.event {
        FrontendEvent::CommitsUpdated { commits } => {
            assert!(commits.len() >= 5);
        }
        _ => panic!("Expected CommitsUpdated event"),
    }

    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_stage_file_command() {
    let test_repo = TestRepo::new();
    test_repo.create_file("unstaged.txt", "content");

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    let response = send_and_receive(
        &cmd_tx,
        &mut event_rx,
        4,
        BackendCommand::StageFile {
            file_path: "unstaged.txt".to_string(),
        },
    )
    .await
    .expect("Should receive response");

    assert_eq!(response.request_id, Some(4));
    match response.event {
        FrontendEvent::ActionSucceeded { message, .. } => {
            assert!(message.contains("Staged"));
        }
        _ => panic!("Expected ActionSucceeded event"),
    }

    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unstage_file_command() {
    let test_repo = TestRepo::with_staged_file("staged.txt", "content");

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    let response = send_and_receive(
        &cmd_tx,
        &mut event_rx,
        5,
        BackendCommand::UnstageFile {
            file_path: "staged.txt".to_string(),
        },
    )
    .await
    .expect("Should receive response");

    assert_eq!(response.request_id, Some(5));
    match response.event {
        FrontendEvent::ActionSucceeded { message, .. } => {
            assert!(message.contains("Unstaged"));
        }
        _ => panic!("Expected ActionSucceeded event"),
    }

    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiple_commands_sequence() {
    let test_repo = TestRepo::new();
    test_repo.create_file("file1.txt", "content1");
    test_repo.create_file("file2.txt", "content2");

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    // Command 1: RefreshStatus
    let resp1 = send_and_receive(&cmd_tx, &mut event_rx, 10, BackendCommand::RefreshStatus)
        .await
        .expect("Should receive response 1");
    assert_eq!(resp1.request_id, Some(10));

    // Command 2: StageFile
    let resp2 = send_and_receive(
        &cmd_tx,
        &mut event_rx,
        11,
        BackendCommand::StageFile {
            file_path: "file1.txt".to_string(),
        },
    )
    .await
    .expect("Should receive response 2");
    assert_eq!(resp2.request_id, Some(11));

    // Command 3: RefreshStatus again
    let resp3 = send_and_receive(&cmd_tx, &mut event_rx, 12, BackendCommand::RefreshStatus)
        .await
        .expect("Should receive response 3");
    assert_eq!(resp3.request_id, Some(12));

    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_revert_commit() {
    let test_repo = TestRepo::new();

    // Create a commit to revert
    test_repo.create_file("revert_me.txt", "content to revert");
    test_repo.stage_file("revert_me.txt");
    test_repo.commit("Commit to revert");

    // Count commits before revert
    let commit_count_before = {
        let mut walk = test_repo.repo.revwalk().expect("revwalk");
        walk.push_head().expect("push_head");
        walk.count()
    };

    // Get the commit OID to revert
    let commit_id = {
        let head = test_repo.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        commit.id().to_string()
    };

    let (cmd_tx, mut event_rx) = setup_backend(test_repo.path.to_str().unwrap()).await;

    let resp = send_and_receive(
        &cmd_tx,
        &mut event_rx,
        42,
        BackendCommand::RevertCommit { commit_id },
    )
    .await
    .expect("Should receive response");

    assert!(
        matches!(resp.event, FrontendEvent::ActionSucceeded { .. }),
        "Expected ActionSucceeded, got {:?}",
        resp.event
    );

    // Verify a new commit was actually created
    let commit_count_after = {
        let mut walk = test_repo.repo.revwalk().expect("revwalk");
        walk.push_head().expect("push_head");
        walk.count()
    };
    assert_eq!(
        commit_count_after,
        commit_count_before + 1,
        "Expected one new commit after revert"
    );

    // Verify the new HEAD message contains "Revert"
    let head_message = {
        let head = test_repo.repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to peel to commit");
        commit.message().unwrap_or("").to_string()
    };
    assert!(
        head_message.contains("Revert"),
        "Expected HEAD message to contain 'Revert', got: {:?}",
        head_message
    );

    cmd_tx.send(CommandEnvelope::new(999, BackendCommand::Quit)).await.ok();
}

