//! Chat screen handler.

use std::sync::Arc;

use tokio::sync::broadcast;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::chat::{
    format_help, format_who, parse_input, ChatCommand, ChatInput, ChatLogRepository, ChatMessage,
    ChatParticipant, ChatRoom, JoinResult, NewChatLog,
};
use crate::error::Result;
use crate::rate_limit::RateLimitResult;
use crate::server::TelnetSession;

/// Maximum length for chat messages (in characters).
pub const MAX_CHAT_MESSAGE_LENGTH: usize = 500;

/// Chat screen handler.
pub struct ChatScreen;

impl ChatScreen {
    /// Run the chat room list screen.
    pub async fn run_list(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<ScreenResult> {
        loop {
            // Display chat room list
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("chat.room_list")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            // Get rooms from manager
            let rooms = ctx.chat_manager.list_rooms().await;

            ctx.send_line(
                session,
                &format!(
                    "  {:<4} {:<20} {}",
                    ctx.i18n.t("common.number"),
                    ctx.i18n.t("chat.room_name"),
                    ctx.i18n.t("chat.users_in_room")
                ),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(40)).await?;

            if rooms.is_empty() {
                ctx.send_line(session, ctx.i18n.t("chat.no_rooms")).await?;
            } else {
                for (i, room) in rooms.iter().enumerate() {
                    ctx.send_line(
                        session,
                        &format!(
                            "  {:<4} {:<20} ({} {})",
                            i + 1,
                            room.name,
                            room.participant_count,
                            ctx.i18n.t("common.people")
                        ),
                    )
                    .await?;
                }
            }

            ctx.send_line(session, "").await?;
            ctx.send(
                session,
                &format!(
                    "{} [Q={}]: ",
                    ctx.i18n.t("menu.select_prompt"),
                    ctx.i18n.t("common.back")
                ),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") || input.is_empty() {
                return Ok(ScreenResult::Back);
            }

            if let Some(num) = ctx.parse_number(input) {
                let idx = (num - 1) as usize;
                if idx < rooms.len() {
                    let room_id = &rooms[idx].id;
                    // Enter the selected room
                    let result = Self::run_room(ctx, session, room_id).await?;
                    if result != ScreenResult::Back {
                        return Ok(result);
                    }
                }
            }
        }
    }

    /// Run a chat room session.
    async fn run_room(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        room_id: &str,
    ) -> Result<ScreenResult> {
        // Get the room
        let room = match ctx.chat_manager.get_room(room_id).await {
            Some(r) => r,
            None => {
                ctx.send_line(session, "Room not found.").await?;
                return Ok(ScreenResult::Back);
            }
        };

        // Get participant info
        let session_id = session.id().to_string();
        let user_id = session.user_id();
        let nickname = session
            .username()
            .map(|s| s.to_string())
            .unwrap_or_else(|| ctx.i18n.t("auth.guest").to_string());

        // Join the room
        let participant = ChatParticipant::new(&session_id, user_id, &nickname);
        match room.join(participant).await {
            JoinResult::Joined => {}
            JoinResult::AlreadyJoined => {}
            JoinResult::RoomFull => {
                ctx.send_line(session, ctx.i18n.t("chat.room_full")).await?;
                return Ok(ScreenResult::Back);
            }
        }

        // Disable auto-paging during chat to prevent "More" prompts
        // from interrupting the message flow
        let saved_auto_paging = ctx.auto_paging_enabled();
        ctx.set_auto_paging(false);

        // Subscribe to messages
        let mut receiver = room.subscribe();

        // Show room header
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", room.name()))
            .await?;
        ctx.send_line(session, ctx.i18n.t("chat.command_help"))
            .await?;
        ctx.send_line(session, "").await?;

        // Show recent logs
        Self::show_recent_logs(ctx, session, room_id).await?;

        // Main chat loop
        let result = Self::chat_loop(
            ctx,
            session,
            &room,
            &mut receiver,
            &session_id,
            user_id,
            &nickname,
        )
        .await;

        // Restore auto-paging setting
        ctx.set_auto_paging(saved_auto_paging);

        // Leave the room
        room.leave(&session_id).await;

        // Leave all rooms when disconnecting
        ctx.chat_manager.leave_all_rooms(&session_id).await;

        result
    }

    /// Show recent chat logs.
    async fn show_recent_logs(
        ctx: &ScreenContext,
        session: &mut TelnetSession,
        room_id: &str,
    ) -> Result<()> {
        let repo = ChatLogRepository::new(ctx.db.pool());
        let logs = repo.get_recent(room_id, 10).await?;

        if !logs.is_empty() {
            ctx.send_line(session, "--- Recent messages ---").await?;
            for log in logs {
                ctx.send_line(session, &log.format()).await?;
            }
            ctx.send_line(session, "---").await?;
            ctx.send_line(session, "").await?;
        }

        Ok(())
    }

    /// Main chat loop.
    async fn chat_loop(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        room: &Arc<ChatRoom>,
        receiver: &mut broadcast::Receiver<ChatMessage>,
        session_id: &str,
        user_id: Option<i64>,
        nickname: &str,
    ) -> Result<ScreenResult> {
        loop {
            // Use select to handle both input and incoming messages
            tokio::select! {
                // Check for incoming messages
                msg_result = receiver.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            // Don't echo our own messages
                            if msg.sender_id.as_deref() != Some(session_id) {
                                ctx.send_line(session, &msg.format()).await?;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            ctx.send_line(session, &format!("*** Missed {} messages", n)).await?;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            ctx.send_line(session, "*** Chat room closed").await?;
                            return Ok(ScreenResult::Back);
                        }
                    }
                }

                // Read user input (with a small timeout to check messages)
                input_result = ctx.read_line_nonblocking(session, 100) => {
                    match input_result {
                        Ok(Some(line)) => {
                            let parsed = parse_input(&line);
                            match parsed {
                                ChatInput::Command(cmd) => {
                                    match cmd {
                                        ChatCommand::Quit => {
                                            return Ok(ScreenResult::Back);
                                        }
                                        ChatCommand::Who => {
                                            let names = room.participant_names().await;
                                            let who_text = format_who(&names, room.name());
                                            for line in who_text.lines() {
                                                ctx.send_line(session, line).await?;
                                            }
                                        }
                                        ChatCommand::Me(action) => {
                                            if !action.is_empty() {
                                                // Check message length
                                                if action.chars().count() > MAX_CHAT_MESSAGE_LENGTH {
                                                    let msg = ctx.i18n.t("chat.message_too_long")
                                                        .replace("{{max}}", &MAX_CHAT_MESSAGE_LENGTH.to_string());
                                                    ctx.send_line(session, &format!("*** {}", msg)).await?;
                                                    continue;
                                                }
                                                // Check rate limit for logged-in users
                                                if let Some(uid) = user_id {
                                                    if let RateLimitResult::Denied { retry_after } = ctx.rate_limiters.chat.check(uid) {
                                                        let msg = ctx.i18n.t_with(
                                                            "rate_limit.chat_denied",
                                                            &[("seconds", &retry_after.as_secs().to_string())],
                                                        );
                                                        ctx.send_line(session, &format!("*** {}", msg)).await?;
                                                        continue;
                                                    }
                                                }
                                                room.send_action(session_id, &action).await;
                                                // Record rate limit for logged-in users
                                                if let Some(uid) = user_id {
                                                    ctx.rate_limiters.chat.record(uid);
                                                }
                                                // Echo to sender
                                                ctx.send_line(session, &format!("* {} {}", nickname, action)).await?;
                                                // Save to log
                                                let log = NewChatLog::action(room.id(), user_id.unwrap_or(0), nickname, &action);
                                                let repo = ChatLogRepository::new(ctx.db.pool());
                                                let _ = repo.save(&log).await;
                                            }
                                        }
                                        ChatCommand::Help => {
                                            let help_text = format_help();
                                            for line in help_text.lines() {
                                                ctx.send_line(session, line).await?;
                                            }
                                        }
                                        ChatCommand::Unknown(cmd_name) => {
                                            ctx.send_line(session, &format!("*** Unknown command: /{}", cmd_name)).await?;
                                        }
                                    }
                                }
                                ChatInput::Message(content) => {
                                    if !content.is_empty() {
                                        // Check message length
                                        if content.chars().count() > MAX_CHAT_MESSAGE_LENGTH {
                                            let msg = ctx.i18n.t("chat.message_too_long")
                                                .replace("{{max}}", &MAX_CHAT_MESSAGE_LENGTH.to_string());
                                            ctx.send_line(session, &format!("*** {}", msg)).await?;
                                            continue;
                                        }
                                        // Check rate limit for logged-in users
                                        if let Some(uid) = user_id {
                                            if let RateLimitResult::Denied { retry_after } = ctx.rate_limiters.chat.check(uid) {
                                                let msg = ctx.i18n.t_with(
                                                    "rate_limit.chat_denied",
                                                    &[("seconds", &retry_after.as_secs().to_string())],
                                                );
                                                ctx.send_line(session, &format!("*** {}", msg)).await?;
                                                continue;
                                            }
                                        }
                                        // Send the message
                                        room.send_message(session_id, &content).await;
                                        // Record rate limit for logged-in users
                                        if let Some(uid) = user_id {
                                            ctx.rate_limiters.chat.record(uid);
                                        }
                                        // Echo to sender
                                        ctx.send_line(session, &format!("<{}> {}", nickname, content)).await?;
                                        // Save to log
                                        let log = NewChatLog::chat(room.id(), user_id.unwrap_or(0), nickname, &content);
                                        let repo = ChatLogRepository::new(ctx.db.pool());
                                        let _ = repo.save(&log).await;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Connection error
                            return Ok(ScreenResult::Quit);
                        }
                        Ok(None) => {
                            // Timeout, just continue to check messages
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_screen_exists() {
        let _ = ChatScreen;
    }

    #[test]
    fn test_max_chat_message_length_is_500() {
        // As specified in CLAUDE.md
        assert_eq!(MAX_CHAT_MESSAGE_LENGTH, 500);
    }

    #[test]
    fn test_message_length_validation_ok() {
        let message = "a".repeat(500);
        assert!(message.chars().count() <= MAX_CHAT_MESSAGE_LENGTH);
    }

    #[test]
    fn test_message_length_validation_too_long() {
        let message = "a".repeat(501);
        assert!(message.chars().count() > MAX_CHAT_MESSAGE_LENGTH);
    }

    #[test]
    fn test_message_length_unicode() {
        // Unicode characters should be counted correctly
        let message = "あ".repeat(500);
        assert_eq!(message.chars().count(), 500);
        assert!(message.chars().count() <= MAX_CHAT_MESSAGE_LENGTH);

        let message_too_long = "あ".repeat(501);
        assert!(message_too_long.chars().count() > MAX_CHAT_MESSAGE_LENGTH);
    }
}
