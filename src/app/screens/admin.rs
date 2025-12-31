//! Admin screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::error::Result;
use crate::server::TelnetSession;

/// Admin screen handler (placeholder).
///
/// Full admin functionality will be implemented in a future phase.
pub struct AdminScreen;

impl AdminScreen {
    /// Run the admin menu.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        // Check admin permission
        if !Self::is_admin(ctx, session) {
            ctx.send_line(session, ctx.i18n.t("menu.admin_required"))
                .await?;
            return Ok(ScreenResult::Back);
        }

        loop {
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("menu.admin")))
                .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.board_management")),
            )
            .await?;
            ctx.send_line(session, "  [1] Board List").await?;
            ctx.send_line(session, "  [2] Create Board").await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.user_management")),
            )
            .await?;
            ctx.send_line(session, "  [3] User List").await?;
            ctx.send_line(session, "  [4] Active Sessions").await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.system_status")),
            )
            .await?;
            ctx.send_line(session, "  [5] System Status").await?;
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
                "1" => Self::show_board_list(ctx, session).await?,
                "2" => Self::create_board(ctx, session).await?,
                "3" => Self::show_user_list(ctx, session).await?,
                "4" => Self::show_sessions(ctx, session).await?,
                "5" => Self::show_system_status(ctx, session).await?,
                _ => {}
            }
        }
    }

    /// Show board list (admin view).
    async fn show_board_list(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::board::BoardRepository;

        let board_repo = BoardRepository::new(&ctx.db);
        let boards = board_repo.list_all()?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.board_management")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if boards.is_empty() {
            ctx.send_line(session, ctx.i18n.t("board.no_boards"))
                .await?;
        } else {
            ctx.send_line(
                session,
                &format!("{:<4} {:<20} {:<10} {:<8}", "ID", "Name", "Type", "Active"),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(50)).await?;

            for board in &boards {
                ctx.send_line(
                    session,
                    &format!(
                        "{:<4} {:<20} {:<10} {:<8}",
                        board.id,
                        board.name,
                        board.board_type.as_str(),
                        if board.is_active { "Yes" } else { "No" }
                    ),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Create a new board.
    async fn create_board(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::board::{BoardRepository, BoardType, NewBoard};

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.create_board")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Get board name
        ctx.send(session, "Board name: ").await?;
        let name = ctx.read_line(session).await?;
        let name = name.trim();

        if name.is_empty() {
            return Ok(());
        }

        // Get board type
        ctx.send(session, "Type (thread/flat) [thread]: ").await?;
        let type_input = ctx.read_line(session).await?;
        let board_type = if type_input.trim().eq_ignore_ascii_case("flat") {
            BoardType::Flat
        } else {
            BoardType::Thread
        };

        // Get description
        ctx.send(session, "Description (optional): ").await?;
        let description = ctx.read_line(session).await?;
        let description = description.trim();

        // Create board
        let board_repo = BoardRepository::new(&ctx.db);
        let new_board = if description.is_empty() {
            NewBoard::new(name).with_board_type(board_type)
        } else {
            NewBoard::new(name)
                .with_board_type(board_type)
                .with_description(description)
        };

        match board_repo.create(&new_board) {
            Ok(board) => {
                ctx.send_line(
                    session,
                    &format!("Board '{}' created (ID: {})", board.name, board.id),
                )
                .await?;
            }
            Err(e) => {
                ctx.send_line(session, &format!("Failed to create board: {}", e))
                    .await?;
            }
        }

        Ok(())
    }

    /// Show user list.
    async fn show_user_list(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::db::UserRepository;

        let user_repo = UserRepository::new(&ctx.db);
        let users = user_repo.list_all()?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.user_list")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if users.is_empty() {
            ctx.send_line(session, "No users found.").await?;
        } else {
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<16} {:<16} {:<8}",
                    "ID", "Username", "Nickname", "Role"
                ),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(50)).await?;

            for user in &users {
                ctx.send_line(
                    session,
                    &format!(
                        "{:<4} {:<16} {:<16} {:?}",
                        user.id, user.username, user.nickname, user.role
                    ),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Show active sessions.
    async fn show_sessions(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.session_list")),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, ctx.i18n.t("feature.not_implemented"))
            .await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Show system status.
    async fn show_system_status(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<()> {
        use crate::board::BoardRepository;
        use crate::db::UserRepository;

        let user_repo = UserRepository::new(&ctx.db);
        let board_repo = BoardRepository::new(&ctx.db);

        let user_count = user_repo.count()?;
        let board_count = board_repo.count()?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.system_status")),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("BBS Name: {}", ctx.config.bbs.name))
            .await?;
        ctx.send_line(session, &format!("SysOp: {}", ctx.config.bbs.sysop_name))
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("Total Users: {}", user_count))
            .await?;
        ctx.send_line(session, &format!("Total Boards: {}", board_count))
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!(
                "Server: {}:{}",
                ctx.config.server.host, ctx.config.server.port
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("Max Connections: {}", ctx.config.server.max_connections),
        )
        .await?;
        ctx.send_line(session, "").await?;

        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Check if user is admin.
    fn is_admin(ctx: &ScreenContext, session: &TelnetSession) -> bool {
        use crate::db::{Role, UserRepository};

        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(&ctx.db);
            if let Ok(Some(user)) = user_repo.get_by_id(user_id) {
                return user.role >= Role::SubOp;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_screen_exists() {
        let _ = AdminScreen;
    }
}
