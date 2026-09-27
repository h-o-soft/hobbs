#![cfg(feature = "sqlite")]
//! E2E test for the admin session list (Issue #130).
//!
//! Waits for expected text instead of fixed delays, so it is not affected by
//! slow password hashing in debug builds (#327).

mod common;

use common::{create_test_user, TestClient, TestServer};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(30);

async fn login(client: &mut TestClient, username: &str) {
    client.recv_until_timeout("Select:", WAIT).await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_until_timeout("Username:", WAIT).await.unwrap();
    client.send_line(username).await.unwrap();
    client.recv_until_timeout("Password:", WAIT).await.unwrap();
    client.send_line("password123").await.unwrap();
    // Main menu prompt after successful login.
    client.recv_until_timeout("> ", WAIT).await.unwrap();
}

#[tokio::test]
async fn test_admin_session_list_shows_connected_users() {
    let server = TestServer::new().await.unwrap();
    create_test_user(server.db(), "sysop", "password123", "sysop")
        .await
        .unwrap();
    create_test_user(server.db(), "alice", "password123", "member")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // A logged-in member and a guest are connected.
    let mut alice = TestClient::connect(server.addr()).await.unwrap();
    login(&mut alice, "alice").await;

    let mut guest = TestClient::connect(server.addr()).await.unwrap();
    guest.recv_until_timeout("Select:", WAIT).await.unwrap();
    guest.send_line("G").await.unwrap();
    guest.recv_until_timeout("> ", WAIT).await.unwrap();
    guest.send_line("E").await.unwrap();
    guest.recv_until_timeout("> ", WAIT).await.unwrap();

    // SysOp opens Admin → [10] session list.
    let mut admin = TestClient::connect(server.addr()).await.unwrap();
    login(&mut admin, "sysop").await;
    admin.send_line("A").await.unwrap();
    // The whole admin menu is shown with its prompt, without an auto-paging
    // pause in the middle (a pause swallowed the "10" typed by the admin).
    let menu = admin.recv_until_timeout("[Q=", WAIT).await.unwrap();
    assert!(!menu.contains("Press ENTER for more"), "{menu}");
    assert!(menu.contains("[10] Active Sessions"), "{menu}");
    admin.send_line("10").await.unwrap();
    let list = admin.recv_until_timeout("[Q=", WAIT).await.unwrap();

    assert!(list.contains("Active Sessions"), "{list}");
    assert!(
        list.contains("sysop"),
        "admin itself should be listed: {list}"
    );
    assert!(list.contains(" *"), "own session should be marked: {list}");
    assert!(
        list.contains("alice"),
        "logged-in member should be listed: {list}"
    );
    assert!(list.contains("(Guest)"), "guest should be listed: {list}");
    assert!(
        list.contains("127.0.0.1"),
        "peer address should be shown: {list}"
    );
    assert!(list.contains("Connected"), "connected-time column: {list}");
    assert!(
        list.contains("0:00:"),
        "connected time is shown as H:MM:SS: {list}"
    );
}
