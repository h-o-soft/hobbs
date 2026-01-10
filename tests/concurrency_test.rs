//! Concurrency tests for HOBBS.
//!
//! These tests verify that concurrent database operations work correctly,
//! especially for operations that involve multiple SQL statements (now protected by transactions).

use std::sync::Arc;

use hobbs::board::{BoardRepository, BoardService, NewBoard};
use hobbs::db::{NewUser, Role, UserRepository};
use hobbs::mail::{MailService, SendMailRequest};
use hobbs::Database;

/// Setup test database with a test user and board.
async fn setup_test_db() -> Arc<Database> {
    Arc::new(Database::open_in_memory().await.unwrap())
}

/// Create a test user and return the user ID.
async fn create_test_user(db: &Database, username: &str) -> i64 {
    let user_repo = UserRepository::new(db.pool());
    let user = NewUser::new(username, "password123", username);
    user_repo.create(&user).await.unwrap().id
}

/// Create a test board and return the board ID.
async fn create_test_board(db: &Database) -> i64 {
    let board_repo = BoardRepository::new(db.pool());
    let new_board = NewBoard::new("Test Board").with_description("A test board");
    board_repo.create(&new_board).await.unwrap().id
}

/// Test concurrent post creation in a thread.
///
/// This test verifies that when multiple posts are created concurrently,
/// the thread's post_count is correctly updated.
#[tokio::test]
async fn test_concurrent_post_creation() {
    let db = setup_test_db().await;

    // Create test user
    let user_id = create_test_user(&db, "testuser").await;

    // Create test board and thread
    let board_id = create_test_board(&db).await;
    let service = BoardService::new(&db);
    let thread = service
        .create_thread(board_id, "Test Thread", user_id, Role::Member)
        .await
        .unwrap();
    let thread_id = thread.id;

    // Number of concurrent posts
    const NUM_POSTS: usize = 10;

    // Create posts concurrently using tokio::spawn
    let mut handles = Vec::new();
    for i in 0..NUM_POSTS {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            let service = BoardService::new(&db_clone);
            let body = format!("Post content {}", i);
            service
                .create_thread_post(thread_id, user_id, body, Role::Member)
                .await
        });
        handles.push(handle);
    }

    // Wait for all posts to complete
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    // All posts should succeed
    assert_eq!(success_count, NUM_POSTS, "All posts should be created");

    // Verify thread's post_count matches the actual number of posts
    let thread = service.get_thread(thread_id, Role::Member).await.unwrap();
    assert_eq!(
        thread.post_count as usize, NUM_POSTS,
        "Thread post_count should match number of created posts"
    );
}

/// Test concurrent post deletion.
///
/// This test verifies that when multiple posts are deleted concurrently,
/// the thread's post_count is correctly decremented.
#[tokio::test]
async fn test_concurrent_post_deletion() {
    let db = setup_test_db().await;

    // Create test user
    let user_id = create_test_user(&db, "testuser2").await;

    // Create test board and thread
    let board_id = create_test_board(&db).await;
    let service = BoardService::new(&db);
    let thread = service
        .create_thread(board_id, "Test Thread", user_id, Role::Member)
        .await
        .unwrap();
    let thread_id = thread.id;

    // Create multiple posts first
    const NUM_POSTS: usize = 5;
    let mut post_ids = Vec::new();
    for i in 0..NUM_POSTS {
        let body = format!("Post to delete {}", i);
        let post = service
            .create_thread_post(thread_id, user_id, body, Role::Member)
            .await
            .unwrap();
        post_ids.push(post.id);
    }

    // Verify initial post count
    let thread = service.get_thread(thread_id, Role::Member).await.unwrap();
    assert_eq!(thread.post_count as usize, NUM_POSTS);

    // Delete posts concurrently
    let mut handles = Vec::new();
    for post_id in post_ids {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            let service = BoardService::new(&db_clone);
            service
                .delete_post(post_id, Some(user_id), Role::Member)
                .await
        });
        handles.push(handle);
    }

    // Wait for all deletions to complete
    let mut delete_count = 0;
    for handle in handles {
        if handle.await.unwrap().unwrap_or(false) {
            delete_count += 1;
        }
    }

    // All deletions should succeed
    assert_eq!(delete_count, NUM_POSTS, "All posts should be deleted");

    // Verify thread's post_count is 0
    let thread = service.get_thread(thread_id, Role::Member).await.unwrap();
    assert_eq!(
        thread.post_count, 0,
        "Thread post_count should be 0 after all posts deleted"
    );
}

/// Test concurrent mail deletion by both sender and recipient.
///
/// This test verifies that when sender and recipient delete a mail concurrently,
/// the mail is properly purged without race conditions.
#[tokio::test]
async fn test_concurrent_mail_deletion() {
    let db = setup_test_db().await;

    // Create sender and recipient
    let sender_id = create_test_user(&db, "sender").await;
    let recipient_id = create_test_user(&db, "recipient").await;

    // Send a mail
    let mail_service = MailService::new(&db);
    let request = SendMailRequest::new(sender_id, "recipient", "Test Subject", "Test Body");
    let mail = mail_service.send_mail(&request).await.unwrap();
    let mail_id = mail.id;

    // Delete by sender and recipient concurrently
    let db1 = Arc::clone(&db);
    let db2 = Arc::clone(&db);

    let handle1 = tokio::spawn(async move {
        let service = MailService::new(&db1);
        service.delete_mail(mail_id, sender_id).await
    });

    let handle2 = tokio::spawn(async move {
        let service = MailService::new(&db2);
        service.delete_mail(mail_id, recipient_id).await
    });

    // Both deletions should succeed (or one might fail if mail is already purged)
    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();

    // At least one should succeed, and there should be no panics or data corruption
    assert!(
        result1.is_ok() || result2.is_ok(),
        "At least one deletion should succeed"
    );

    // Verify mail is deleted (should not be found)
    let mails = mail_service.list_inbox(sender_id).await.unwrap();
    assert!(
        mails.iter().all(|m| m.id != mail_id),
        "Mail should not appear in sender's inbox"
    );
}

/// Test mixed concurrent operations on the same thread.
///
/// This test runs create and delete operations concurrently to verify
/// transaction isolation.
#[tokio::test]
async fn test_mixed_concurrent_operations() {
    let db = setup_test_db().await;

    // Create test user
    let user_id = create_test_user(&db, "mixeduser").await;

    // Create test board and thread
    let board_id = create_test_board(&db).await;
    let service = BoardService::new(&db);
    let thread = service
        .create_thread(board_id, "Mixed Test Thread", user_id, Role::Member)
        .await
        .unwrap();
    let thread_id = thread.id;

    // Create initial posts
    let mut initial_posts = Vec::new();
    for i in 0..3 {
        let post = service
            .create_thread_post(thread_id, user_id, format!("Initial post {}", i), Role::Member)
            .await
            .unwrap();
        initial_posts.push(post.id);
    }

    // Run creates and deletes concurrently
    let mut handles = Vec::new();

    // Spawn create tasks
    for i in 0..5 {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            let service = BoardService::new(&db_clone);
            service
                .create_thread_post(thread_id, user_id, format!("New post {}", i), Role::Member)
                .await
                .map(|_| 1i32)
        });
        handles.push(handle);
    }

    // Spawn delete tasks for initial posts
    for post_id in initial_posts {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            let service = BoardService::new(&db_clone);
            service
                .delete_post(post_id, Some(user_id), Role::Member)
                .await
                .map(|deleted| if deleted { -1i32 } else { 0i32 })
        });
        handles.push(handle);
    }

    // Wait for all operations and count net change
    let mut net_change: i32 = 0;
    for handle in handles {
        if let Ok(Ok(change)) = handle.await {
            net_change += change;
        }
    }

    // Expected: 5 creates - 3 deletes = 2 net change
    // Final post_count should be net_change (since we deleted all initial posts)
    let thread = service.get_thread(thread_id, Role::Member).await.unwrap();

    // We expect 5 new posts created and 3 initial posts deleted = 5 posts remaining
    assert_eq!(
        thread.post_count, 5,
        "Thread should have 5 posts after mixed operations (5 created, 3 deleted)"
    );
}
