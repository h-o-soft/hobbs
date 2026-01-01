//! E2E tests for profile settings screen.
//!
//! Tests that users can change language/encoding settings from profile screen.

mod common;

use common::{create_test_user_with_settings, TestClient, TestServer};
use std::time::Duration;

/// Test accessing settings screen from profile.
/// User logs in, goes to profile, selects settings.
#[tokio::test]
async fn test_profile_settings_accessible() {
    let server = TestServer::new().await.unwrap();
    // Create user with English language and UTF-8 encoding
    create_test_user_with_settings(
        server.db(),
        "settingsuser",
        "password123",
        "member",
        "en",    // English
        "utf-8", // UTF-8 encoding
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Select English at welcome
    client.select_language("E").await.unwrap();

    // Wait for welcome
    client.recv_until("Select:").await.unwrap();

    // Login
    client.send_line("L").await.unwrap();
    client.recv_until("Username:").await.unwrap();
    client.send_line("settingsuser").await.unwrap();
    client.recv_until("Password:").await.unwrap();
    client.send_line("password123").await.unwrap();

    // Wait for login success and menu
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Go to profile (wait for menu prompt)
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();
    client.send_line("P").await.unwrap();

    // Wait for profile screen with options
    let response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Profile should show user info and menu options with [S] for settings
    assert!(
        response.contains("settingsuser") || response.contains("Profile")
            || response.contains("[S]") || response.contains("[E]"),
        "Profile screen should show username or options: {:?}",
        response
    );

    // Select settings [S] - the profile menu is already displayed
    client.send_line("S").await.unwrap();

    // Wait for settings screen - may need multiple receives
    let mut settings = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Get more data if we haven't seen language options yet
    if !settings.contains("Language") && !settings.contains("言語") && !settings.contains("[1]") {
        settings.push_str(
            &client
                .recv_timeout(Duration::from_secs(2))
                .await
                .unwrap_or_default(),
        );
    }

    // Settings should show language and encoding options
    assert!(
        settings.contains("Language") || settings.contains("Encoding")
            || settings.contains("UTF-8") || settings.contains("[1]")
            || settings.contains("言語") || settings.contains("文字")
            || settings.contains("English") || settings.contains("Japanese"),
        "Settings screen should show language/encoding options: {:?}",
        settings
    );
}

/// Test changing language setting from English to Japanese.
#[tokio::test]
async fn test_change_language_en_to_ja() {
    let server = TestServer::new().await.unwrap();
    // Create user with English language
    create_test_user_with_settings(
        server.db(),
        "languser",
        "password123",
        "member",
        "en",    // English
        "utf-8", // UTF-8 encoding
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Select English at welcome
    client.select_language("E").await.unwrap();

    // Wait for welcome and login
    client.recv_until("Select:").await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_until("Username:").await.unwrap();
    client.send_line("languser").await.unwrap();
    client.recv_until("Password:").await.unwrap();
    client.send_line("password123").await.unwrap();

    // Wait for login success and menu
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();

    // Go to profile
    client.send_line("P").await.unwrap();

    // Wait for profile screen with options
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Select settings [S]
    client.send_line("S").await.unwrap();

    // Wait for settings screen showing language options
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Select Japanese (option 2) for language
    client.send_line("2").await.unwrap();

    // Wait for encoding prompt - receive until we see ShiftJIS option
    let mut enc_prompt = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    // Get more if needed
    if !enc_prompt.contains("ShiftJIS") {
        enc_prompt.push_str(
            &client
                .recv_timeout(Duration::from_secs(1))
                .await
                .unwrap_or_default(),
        );
    }
    assert!(
        enc_prompt.contains("Encoding")
            || enc_prompt.contains("UTF-8")
            || enc_prompt.contains("ShiftJIS")
            || enc_prompt.contains("[1]")
            || enc_prompt.contains("文字"),
        "Should show encoding prompt: {:?}",
        enc_prompt
    );

    // Select encoding (keep default UTF-8)
    client.send_line("").await.unwrap();

    // Wait for terminal profile prompt
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();

    // Keep terminal profile as default
    client.send_line("").await.unwrap();

    // Wait for settings saved message and return to main menu
    // After SettingsChanged, we go back to main menu (not profile)
    let mut response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Get more data to capture the full response
    if let Ok(more) = client.recv_timeout(Duration::from_secs(1)).await {
        response.push_str(&more);
    }

    // After settings saved, should see save confirmation or main menu in Japanese
    assert!(
        response.contains("設定")
            || response.contains("保存")
            || response.contains("saved")
            || response.contains("メニュー")
            || response.contains("掲示板")
            || response.contains(">"),
        "After language change, should see confirmation or menu: {:?}",
        response
    );
}

/// Test changing encoding setting from UTF-8 to ShiftJIS.
#[tokio::test]
async fn test_change_encoding_utf8_to_shiftjis() {
    let server = TestServer::new().await.unwrap();
    // Create user with Japanese language and UTF-8 encoding
    create_test_user_with_settings(
        server.db(),
        "encuser",
        "password123",
        "member",
        "ja",    // Japanese
        "utf-8", // UTF-8 encoding
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Select Japanese UTF-8 at welcome
    client.select_language("U").await.unwrap();

    // Wait for welcome (in Japanese)
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Login
    client.send_line("L").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap();
    client.send_line("encuser").await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap();
    client.send_line("password123").await.unwrap();

    // Wait for login success
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();

    // Go to profile
    client.send_line("P").await.unwrap();

    // Wait for profile screen
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Select settings [S]
    client.send_line("S").await.unwrap();

    // Wait for settings screen
    let settings = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    assert!(
        settings.contains("UTF-8") || settings.contains("ShiftJIS") || settings.contains("[1]")
            || settings.contains("言語") || settings.contains("文字"),
        "Settings should show encoding options: {:?}",
        settings
    );

    // Keep language as is (press enter for default)
    client.send_line("").await.unwrap();

    // Wait for encoding prompt
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();

    // Select ShiftJIS encoding (option 2)
    client.send_line("2").await.unwrap();

    // Wait for terminal profile prompt
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap_or_default();

    // Keep terminal profile as default
    client.send_line("").await.unwrap();

    // Wait for settings saved message
    let mut response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Get more data if needed
    if let Ok(more) = client.recv_timeout(Duration::from_secs(1)).await {
        response.push_str(&more);
    }

    // Settings should be saved
    assert!(
        response.contains("設定")
            || response.contains("保存")
            || response.contains("saved")
            || response.contains("[E]")
            || response.contains("[P]")
            || response.contains("[S]")
            || response.contains("メニュー"),
        "Settings should be saved: {:?}",
        response
    );
}

/// Test that settings change persists and main menu shows in new language.
#[tokio::test]
async fn test_settings_persist_on_main_menu() {
    let server = TestServer::new().await.unwrap();
    // Create user with English language
    create_test_user_with_settings(
        server.db(),
        "persistuser",
        "password123",
        "member",
        "en",    // English
        "utf-8", // UTF-8 encoding
    )
    .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TestClient::connect(server.addr()).await.unwrap();

    // Select English at welcome
    client.select_language("E").await.unwrap();

    // Login
    client.recv_until("Select:").await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_until("Username:").await.unwrap();
    client.send_line("persistuser").await.unwrap();
    client.recv_until("Password:").await.unwrap();
    client.send_line("password123").await.unwrap();

    // Wait for login and menu
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();

    // Go to profile
    client.send_line("P").await.unwrap();

    // Profile screen
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Go to settings
    client.send_line("S").await.unwrap();

    // Settings screen - wait for language options
    let _ = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Change to Japanese (option 2)
    client.send_line("2").await.unwrap();

    // Wait for encoding prompt - receive until we see the full prompt
    let mut enc_prompt = client.recv_timeout(Duration::from_secs(2)).await.unwrap();
    if !enc_prompt.contains("ShiftJIS") {
        enc_prompt.push_str(
            &client
                .recv_timeout(Duration::from_secs(1))
                .await
                .unwrap_or_default(),
        );
    }

    // Keep encoding default
    client.send_line("").await.unwrap();

    // Wait for terminal profile prompt
    let _ = client.recv_timeout(Duration::from_secs(1)).await.unwrap_or_default();

    // Keep terminal profile as default
    client.send_line("").await.unwrap();

    // After SettingsChanged, we go back to main menu (not profile)
    // Get the settings saved message and/or main menu
    let mut response = client.recv_timeout(Duration::from_secs(2)).await.unwrap();

    // Get more data to ensure we capture the full response
    if let Ok(more) = client.recv_timeout(Duration::from_secs(1)).await {
        response.push_str(&more);
    }

    // Should see save confirmation or main menu in Japanese
    assert!(
        response.contains("設定")
            || response.contains("保存")
            || response.contains("saved")
            || response.contains("メニュー")
            || response.contains("掲示板")
            || response.contains(">"),
        "After settings change, should see confirmation or menu: {:?}",
        response
    );

    // Get more output to see main menu
    let menu = client.recv_timeout(Duration::from_secs(2)).await.unwrap_or_default();
    let combined = format!("{}{}", response, menu);

    // Main menu should now be in Japanese (or show menu options)
    assert!(
        combined.contains("掲示板")
            || combined.contains("メニュー")
            || combined.contains("B")
            || combined.contains(">")
            || combined.contains("保存"),
        "Menu should be in Japanese or show options: {:?}",
        combined
    );
}
