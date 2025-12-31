//! Chat screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::error::Result;
use crate::server::TelnetSession;

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

            // Static room list for now (will be managed dynamically later)
            let rooms = ["Lobby", "Tech", "Random"];

            ctx.send_line(
                session,
                &format!(
                    "  {:<4} {:<20}",
                    ctx.i18n.t("common.number"),
                    ctx.i18n.t("chat.room_name")
                ),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(30)).await?;

            for (i, room) in rooms.iter().enumerate() {
                ctx.send_line(session, &format!("  {:<4} {:<20}", i + 1, room))
                    .await?;
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
                    // For now, show "not implemented" message
                    ctx.send_line(session, "").await?;
                    ctx.send_line(session, ctx.i18n.t("feature.not_implemented"))
                        .await?;
                    ctx.send_line(
                        session,
                        "Chat functionality requires ChatRoomManager integration.",
                    )
                    .await?;
                    ctx.wait_for_enter(session).await?;
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
}
