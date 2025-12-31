//! Help screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::error::Result;
use crate::server::TelnetSession;

/// Help screen handler.
pub struct HelpScreen;

impl HelpScreen {
    /// Run the help screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        loop {
            // Display help menu
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("common.help")))
                .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(session, "  [1] About HOBBS").await?;
            ctx.send_line(session, "  [2] Navigation").await?;
            ctx.send_line(session, "  [3] Board Commands").await?;
            ctx.send_line(session, "  [4] Chat Commands").await?;
            ctx.send_line(session, "  [5] Mail Commands").await?;
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

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "1" => Self::show_about(ctx, session).await?,
                "2" => Self::show_navigation(ctx, session).await?,
                "3" => Self::show_board_help(ctx, session).await?,
                "4" => Self::show_chat_help(ctx, session).await?,
                "5" => Self::show_mail_help(ctx, session).await?,
                _ => {}
            }
        }
    }

    /// Show about screen.
    async fn show_about(ctx: &ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "=== About HOBBS ===").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "HOBBS - Hobbyist Bulletin Board System")
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            "A retro-style BBS system that brings back the feel of",
        )
        .await?;
        ctx.send_line(
            session,
            "classic bulletin board systems from the 80s and 90s.",
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Features:").await?;
        ctx.send_line(session, "  - Message boards (thread and flat formats)")
            .await?;
        ctx.send_line(session, "  - Real-time chat rooms").await?;
        ctx.send_line(session, "  - Private mail between users")
            .await?;
        ctx.send_line(session, "  - File library").await?;
        ctx.send_line(session, "  - User profiles").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("SysOp: {}", ctx.config.bbs.sysop_name))
            .await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Show navigation help.
    async fn show_navigation(ctx: &ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "=== Navigation ===").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Common Commands:").await?;
        ctx.send_line(session, "  Q   - Go back / Quit current screen")
            .await?;
        ctx.send_line(session, "  N   - Next page").await?;
        ctx.send_line(session, "  P   - Previous page").await?;
        ctx.send_line(session, "  #   - Select item by number")
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Main Menu:").await?;
        ctx.send_line(session, "  B   - Boards").await?;
        ctx.send_line(session, "  C   - Chat").await?;
        ctx.send_line(session, "  M   - Mail").await?;
        ctx.send_line(session, "  F   - Files").await?;
        ctx.send_line(session, "  P   - Profile").await?;
        ctx.send_line(session, "  H   - Help").await?;
        ctx.send_line(session, "  Q   - Logout").await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Show board help.
    async fn show_board_help(ctx: &ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "=== Board Commands ===").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Board List:").await?;
        ctx.send_line(session, "  #   - Open board by number")
            .await?;
        ctx.send_line(session, "  Q   - Return to main menu")
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Thread/Post List:").await?;
        ctx.send_line(session, "  #   - Open thread/post by number")
            .await?;
        ctx.send_line(session, "  W   - Write new thread/post")
            .await?;
        ctx.send_line(session, "  N   - Next page").await?;
        ctx.send_line(session, "  P   - Previous page").await?;
        ctx.send_line(session, "  Q   - Return to board list")
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Reading Posts:").await?;
        ctx.send_line(session, "  R   - Reply to thread").await?;
        ctx.send_line(session, "  N   - Next page").await?;
        ctx.send_line(session, "  P   - Previous page").await?;
        ctx.send_line(session, "  Q   - Return to list").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Writing:").await?;
        ctx.send_line(session, "  Enter text line by line").await?;
        ctx.send_line(session, "  Type '.' on a line by itself to finish")
            .await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Show chat help.
    async fn show_chat_help(ctx: &ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "=== Chat Commands ===").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "In Chat Room:").await?;
        ctx.send_line(session, "  /quit   - Leave the room").await?;
        ctx.send_line(session, "  /who    - List participants")
            .await?;
        ctx.send_line(session, "  /me     - Action message (e.g., /me waves)")
            .await?;
        ctx.send_line(session, "  /help   - Show help").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Normal text is sent as a message to everyone.")
            .await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Show mail help.
    async fn show_mail_help(ctx: &ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "=== Mail Commands ===").await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Mail List:").await?;
        ctx.send_line(session, "  #   - Read mail by number")
            .await?;
        ctx.send_line(session, "  W   - Write new mail").await?;
        ctx.send_line(session, "  N   - Next page").await?;
        ctx.send_line(session, "  P   - Previous page").await?;
        ctx.send_line(session, "  Q   - Return to main menu")
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Reading Mail:").await?;
        ctx.send_line(session, "  R   - Reply to sender").await?;
        ctx.send_line(session, "  D   - Delete mail").await?;
        ctx.send_line(session, "  N   - Next mail").await?;
        ctx.send_line(session, "  P   - Previous mail").await?;
        ctx.send_line(session, "  Q   - Return to list").await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_screen_exists() {
        let _ = HelpScreen;
    }
}
