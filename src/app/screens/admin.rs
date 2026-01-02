//! Admin screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::chat::DeleteRoomError;
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
            ctx.send_line(
                session,
                &format!("  [1] {}", ctx.i18n.t("admin.board_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [2] {}", ctx.i18n.t("admin.create_board")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [3] {}", ctx.i18n.t("admin.edit_board")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [4] {}", ctx.i18n.t("admin.delete_board")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.user_management")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [5] {}", ctx.i18n.t("admin.user_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [6] {}", ctx.i18n.t("admin.session_list")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.chat_management")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [7] {}", ctx.i18n.t("admin.chat_room_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [8] {}", ctx.i18n.t("admin.create_chat_room")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [9] {}", ctx.i18n.t("admin.delete_chat_room")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.file_management")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [10] {}", ctx.i18n.t("admin.folder_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [11] {}", ctx.i18n.t("admin.create_folder")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [12] {}", ctx.i18n.t("admin.delete_folder")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.system_status")),
            )
            .await?;
            ctx.send_line(session, "  [13] System Status").await?;
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
                "3" => Self::edit_board(ctx, session).await?,
                "4" => Self::delete_board(ctx, session).await?,
                "5" => Self::show_user_list(ctx, session).await?,
                "6" => Self::show_sessions(ctx, session).await?,
                "7" => Self::show_chat_rooms(ctx, session).await?,
                "8" => Self::create_chat_room(ctx, session).await?,
                "9" => Self::delete_chat_room(ctx, session).await?,
                "10" => Self::show_folders(ctx, session).await?,
                "11" => Self::create_folder(ctx, session).await?,
                "12" => Self::delete_folder(ctx, session).await?,
                "13" => Self::show_system_status(ctx, session).await?,
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

    /// Edit a board (permissions, name, description, etc.).
    async fn edit_board(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::board::{BoardRepository, BoardUpdate};

        // Get board list
        let boards = {
            let board_repo = BoardRepository::new(&ctx.db);
            board_repo.list_all()?
        };

        if boards.is_empty() {
            ctx.send_line(session, ctx.i18n.t("board.no_boards")).await?;
            return Ok(());
        }

        // Show board list
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.edit_board")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        for (i, board) in boards.iter().enumerate() {
            let status = if board.is_active {
                ctx.i18n.t("admin.board_active")
            } else {
                ctx.i18n.t("admin.board_inactive")
            };
            ctx.send_line(
                session,
                &format!(
                    "  {}: {} [{}]",
                    i + 1,
                    board.name,
                    status
                ),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("admin.board_number_to_edit")),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.is_empty() {
            return Ok(());
        }

        let idx: usize = match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= boards.len() => n - 1,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        let board_id = boards[idx].id;

        // Show current settings and edit menu
        loop {
            // Reload board to get latest data
            let board = {
                let board_repo = BoardRepository::new(&ctx.db);
                match board_repo.get_by_id(board_id)? {
                    Some(b) => b,
                    None => {
                        ctx.send_line(session, ctx.i18n.t("admin.board_not_found"))
                            .await?;
                        return Ok(());
                    }
                }
            };

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.board_current_settings")),
            )
            .await?;

            let read_role_name = Self::role_to_string(&board.min_read_role, ctx);
            let write_role_name = Self::role_to_string(&board.min_write_role, ctx);
            let status = if board.is_active {
                ctx.i18n.t("admin.board_active")
            } else {
                ctx.i18n.t("admin.board_inactive")
            };

            ctx.send_line(
                session,
                &format!(
                    "  {}: {}",
                    ctx.i18n.t("admin.board_name"),
                    board.name
                ),
            )
            .await?;
            ctx.send_line(
                session,
                &format!(
                    "  {}: {}",
                    ctx.i18n.t("admin.board_description"),
                    board.description.as_deref().unwrap_or("-")
                ),
            )
            .await?;
            ctx.send_line(
                session,
                &format!(
                    "  {}: {}",
                    ctx.i18n.t("admin.board_read_permission"),
                    read_role_name
                ),
            )
            .await?;
            ctx.send_line(
                session,
                &format!(
                    "  {}: {}",
                    ctx.i18n.t("admin.board_write_permission"),
                    write_role_name
                ),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  {}: {}", ctx.i18n.t("admin.board_active"), status),
            )
            .await?;

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.board_select_item")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [1] {}", ctx.i18n.t("admin.board_edit_name")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [2] {}", ctx.i18n.t("admin.board_edit_description")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [3] {}", ctx.i18n.t("admin.board_edit_read_permission")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [4] {}", ctx.i18n.t("admin.board_edit_write_permission")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [5] {}", ctx.i18n.t("admin.board_edit_active")),
            )
            .await?;
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

            let choice = ctx.read_line(session).await?;
            let choice = choice.trim();

            match choice.to_ascii_lowercase().as_str() {
                "q" | "" => break,
                "1" => {
                    // Edit name
                    ctx.send(
                        session,
                        &format!("{}: ", ctx.i18n.t("admin.new_name")),
                    )
                    .await?;
                    let new_name = ctx.read_line(session).await?;
                    let new_name = new_name.trim();
                    if !new_name.is_empty() {
                        let update = BoardUpdate::new().name(new_name);
                        let board_repo = BoardRepository::new(&ctx.db);
                        if let Err(e) = board_repo.update(board_id, &update) {
                            ctx.send_line(session, &format!("Error: {}", e)).await?;
                        } else {
                            ctx.send_line(
                                session,
                                &ctx.i18n.t_with("admin.board_updated", &[("name", new_name)]),
                            )
                            .await?;
                        }
                    }
                }
                "2" => {
                    // Edit description
                    ctx.send(
                        session,
                        &format!("{}: ", ctx.i18n.t("admin.new_description")),
                    )
                    .await?;
                    let new_desc = ctx.read_line(session).await?;
                    let new_desc = new_desc.trim();
                    let update = if new_desc.is_empty() {
                        BoardUpdate::new().description(None)
                    } else {
                        BoardUpdate::new().description(Some(new_desc.to_string()))
                    };
                    let board_repo = BoardRepository::new(&ctx.db);
                    if let Err(e) = board_repo.update(board_id, &update) {
                        ctx.send_line(session, &format!("Error: {}", e)).await?;
                    } else {
                        ctx.send_line(
                            session,
                            &ctx.i18n.t_with("admin.board_updated", &[("name", &board.name)]),
                        )
                        .await?;
                    }
                }
                "3" => {
                    // Edit read permission
                    if let Some(role) = Self::select_role(ctx, session).await? {
                        let update = BoardUpdate::new().min_read_role(role);
                        let board_repo = BoardRepository::new(&ctx.db);
                        if let Err(e) = board_repo.update(board_id, &update) {
                            ctx.send_line(session, &format!("Error: {}", e)).await?;
                        } else {
                            ctx.send_line(
                                session,
                                &ctx.i18n.t_with("admin.board_updated", &[("name", &board.name)]),
                            )
                            .await?;
                        }
                    }
                }
                "4" => {
                    // Edit write permission
                    if let Some(role) = Self::select_role(ctx, session).await? {
                        let update = BoardUpdate::new().min_write_role(role);
                        let board_repo = BoardRepository::new(&ctx.db);
                        if let Err(e) = board_repo.update(board_id, &update) {
                            ctx.send_line(session, &format!("Error: {}", e)).await?;
                        } else {
                            ctx.send_line(
                                session,
                                &ctx.i18n.t_with("admin.board_updated", &[("name", &board.name)]),
                            )
                            .await?;
                        }
                    }
                }
                "5" => {
                    // Toggle active status
                    let update = BoardUpdate::new().is_active(!board.is_active);
                    let board_repo = BoardRepository::new(&ctx.db);
                    if let Err(e) = board_repo.update(board_id, &update) {
                        ctx.send_line(session, &format!("Error: {}", e)).await?;
                    } else {
                        ctx.send_line(
                            session,
                            &ctx.i18n.t_with("admin.board_updated", &[("name", &board.name)]),
                        )
                        .await?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Convert Role to localized string.
    fn role_to_string(role: &crate::db::Role, ctx: &ScreenContext) -> String {
        match role {
            crate::db::Role::Guest => ctx.i18n.t("role.guest").to_string(),
            crate::db::Role::Member => ctx.i18n.t("role.member").to_string(),
            crate::db::Role::SubOp => ctx.i18n.t("role.subop").to_string(),
            crate::db::Role::SysOp => ctx.i18n.t("role.sysop").to_string(),
        }
    }

    /// Show role selection menu and return selected role.
    async fn select_role(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<Option<crate::db::Role>> {
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.select_permission")),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [1] {} ({})", ctx.i18n.t("role.guest"), "Guest"),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [2] {} ({})", ctx.i18n.t("role.member"), "Member"),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [3] {} ({})", ctx.i18n.t("role.subop"), "SubOp"),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [4] {} ({})", ctx.i18n.t("role.sysop"), "SysOp"),
        )
        .await?;
        ctx.send_line(session, "").await?;

        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("menu.select_prompt"),
                ctx.i18n.t("common.cancel")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        match input.to_ascii_lowercase().as_str() {
            "1" => Ok(Some(crate::db::Role::Guest)),
            "2" => Ok(Some(crate::db::Role::Member)),
            "3" => Ok(Some(crate::db::Role::SubOp)),
            "4" => Ok(Some(crate::db::Role::SysOp)),
            _ => Ok(None),
        }
    }

    /// Delete a board.
    async fn delete_board(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::admin::BoardAdminService;
        use crate::board::{BoardRepository, PostRepository, ThreadRepository};
        use crate::db::{Role, UserRepository};

        // Check SysOp permission
        let user_id = match session.user_id() {
            Some(id) => id,
            None => {
                ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                    .await?;
                return Ok(());
            }
        };

        let user_repo = UserRepository::new(&ctx.db);
        let user = match user_repo.get_by_id(user_id)? {
            Some(u) => u,
            None => return Ok(()),
        };

        if user.role != Role::SysOp {
            ctx.send_line(session, ctx.i18n.t("admin.sysop_required"))
                .await?;
            return Ok(());
        }

        // Show board list
        let board_repo = BoardRepository::new(&ctx.db);
        let boards = board_repo.list_all()?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.delete_board")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if boards.is_empty() {
            ctx.send_line(session, ctx.i18n.t("board.no_boards"))
                .await?;
            return Ok(());
        }

        // Get thread/post counts for all boards upfront
        let board_counts: Vec<(i64, i64)> = {
            let thread_repo = ThreadRepository::new(&ctx.db);
            let post_repo = PostRepository::new(&ctx.db);
            boards
                .iter()
                .map(|b| {
                    let t = thread_repo.count_by_board(b.id).unwrap_or(0);
                    let p = post_repo.count_by_board(b.id).unwrap_or(0);
                    (t, p)
                })
                .collect()
        };

        // Display boards with thread/post counts
        ctx.send_line(
            session,
            &format!(
                "  {:<4} {:<20} {:<10} {:<10}",
                ctx.i18n.t("common.number"),
                ctx.i18n.t("board.title"),
                ctx.i18n.t("board.replies"),
                ctx.i18n.t("board.views")
            ),
        )
        .await?;
        ctx.send_line(session, &"-".repeat(50)).await?;

        for (i, board) in boards.iter().enumerate() {
            let (thread_count, post_count) = board_counts[i];
            ctx.send_line(
                session,
                &format!(
                    "  {:<4} {:<20} {:<10} {:<10}",
                    i + 1,
                    if board.name.chars().count() > 18 {
                        format!("{}...", board.name.chars().take(15).collect::<String>())
                    } else {
                        board.name.clone()
                    },
                    thread_count,
                    post_count
                ),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;

        // Get board number to delete
        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("admin.board_number_to_delete"),
                ctx.i18n.t("common.cancel")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.is_empty() || input.eq_ignore_ascii_case("q") {
            return Ok(());
        }

        let board_num: usize = match input.parse() {
            Ok(n) if n > 0 && n <= boards.len() => n,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        let board = &boards[board_num - 1];
        let (thread_count, post_count) = board_counts[board_num - 1];

        // Show confirmation
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &ctx.i18n.t_with(
                "admin.board_delete_confirm",
                &[
                    ("name", &board.name),
                    ("threads", &thread_count.to_string()),
                    ("posts", &post_count.to_string()),
                ],
            ),
        )
        .await?;
        ctx.send(session, "[Y/N]: ").await?;

        let confirm = ctx.read_line(session).await?;
        if !confirm.trim().eq_ignore_ascii_case("y") {
            ctx.send_line(session, ctx.i18n.t("common.cancel")).await?;
            return Ok(());
        }

        // Delete board
        let admin_service = BoardAdminService::new(&ctx.db);
        match admin_service.delete_board(board.id, &user) {
            Ok(true) => {
                ctx.send_line(
                    session,
                    &ctx.i18n.t_with("admin.board_deleted", &[("name", &board.name)]),
                )
                .await?;
            }
            Ok(false) => {
                ctx.send_line(session, ctx.i18n.t("admin.board_not_found"))
                    .await?;
            }
            Err(e) => {
                ctx.send_line(session, &format!("{}: {}", ctx.i18n.t("common.error"), e))
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

    /// Show chat room list.
    async fn show_chat_rooms(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        let rooms = ctx.chat_manager.list_rooms().await;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.chat_room_list")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if rooms.is_empty() {
            ctx.send_line(session, ctx.i18n.t("chat.no_rooms")).await?;
        } else {
            ctx.send_line(
                session,
                &format!("{:<12} {:<20} {}", "ID", "Name", "Users"),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(40)).await?;

            for room in &rooms {
                ctx.send_line(
                    session,
                    &format!("{:<12} {:<20} {}", room.id, room.name, room.participant_count),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Create a new chat room.
    async fn create_chat_room(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.create_chat_room")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Get room ID
        ctx.send(session, "Room ID: ").await?;
        let id = ctx.read_line(session).await?;
        let id = id.trim();

        if id.is_empty() {
            return Ok(());
        }

        // Get room name
        ctx.send(session, "Room Name: ").await?;
        let name = ctx.read_line(session).await?;
        let name = name.trim();

        if name.is_empty() {
            return Ok(());
        }

        // Create the room
        match ctx.chat_manager.create_room(id, name).await {
            Some(_) => {
                let msg = ctx
                    .i18n
                    .t("admin.room_created")
                    .replace("{{name}}", name);
                ctx.send_line(session, &msg).await?;
            }
            None => {
                ctx.send_line(session, ctx.i18n.t("admin.room_id_exists"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Delete a chat room.
    async fn delete_chat_room(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<()> {
        // Show current rooms
        let rooms = ctx.chat_manager.list_rooms().await;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.delete_chat_room")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if rooms.is_empty() {
            ctx.send_line(session, ctx.i18n.t("chat.no_rooms")).await?;
            ctx.send_line(session, "").await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Show room list
        for (i, room) in rooms.iter().enumerate() {
            ctx.send_line(
                session,
                &format!(
                    "  [{}] {} ({}) - {} users",
                    i + 1,
                    room.name,
                    room.id,
                    room.participant_count
                ),
            )
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
            return Ok(());
        }

        if let Some(num) = ctx.parse_number(input) {
            let idx = (num - 1) as usize;
            if idx < rooms.len() {
                let room = &rooms[idx];
                match ctx.chat_manager.delete_room(&room.id).await {
                    Ok(name) => {
                        let msg = ctx
                            .i18n
                            .t("admin.room_deleted")
                            .replace("{{name}}", &name);
                        ctx.send_line(session, &msg).await?;
                    }
                    Err(DeleteRoomError::NotFound) => {
                        ctx.send_line(session, ctx.i18n.t("admin.room_not_found"))
                            .await?;
                    }
                    Err(DeleteRoomError::HasParticipants) => {
                        ctx.send_line(session, ctx.i18n.t("admin.room_has_users"))
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Show folder list.
    async fn show_folders(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::file::FolderRepository;

        let folders = FolderRepository::list_root(ctx.db.conn())?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.folder_list")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if folders.is_empty() {
            ctx.send_line(session, ctx.i18n.t("file.no_folders")).await?;
        } else {
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<20} {:<10} {:<10}",
                    "ID",
                    ctx.i18n.t("file.folder_list"),
                    ctx.i18n.t("admin.permission"),
                    ctx.i18n.t("admin.upload_perm")
                ),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(50)).await?;

            for folder in &folders {
                let file_count = FolderRepository::count_files(ctx.db.conn(), folder.id)?;
                ctx.send_line(
                    session,
                    &format!(
                        "{:<4} {:<20} {:<10} {:<10} ({} files)",
                        folder.id,
                        folder.name,
                        folder.permission.as_str(),
                        folder.upload_perm.as_str(),
                        file_count
                    ),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Create a new folder.
    async fn create_folder(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::db::Role;
        use crate::file::{FolderRepository, NewFolder};

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.create_folder")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Get folder name
        ctx.send(session, &format!("{}: ", ctx.i18n.t("admin.folder_name")))
            .await?;
        let name = ctx.read_line(session).await?;
        let name = name.trim();

        if name.is_empty() {
            return Ok(());
        }

        // Get description
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("file.description")),
        )
        .await?;
        let description = ctx.read_line(session).await?;
        let description = description.trim();

        // Get view permission
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "View permission:").await?;
        ctx.send_line(session, "  [1] Guest").await?;
        ctx.send_line(session, "  [2] Member").await?;
        ctx.send_line(session, "  [3] SubOp").await?;
        ctx.send_line(session, "  [4] SysOp").await?;
        ctx.send(session, "Select [2]: ").await?;
        let perm_input = ctx.read_line(session).await?;
        let permission = match perm_input.trim() {
            "1" => Role::Guest,
            "3" => Role::SubOp,
            "4" => Role::SysOp,
            _ => Role::Member,
        };

        // Get upload permission
        ctx.send_line(session, "").await?;
        ctx.send_line(session, "Upload permission:").await?;
        ctx.send_line(session, "  [1] Guest").await?;
        ctx.send_line(session, "  [2] Member").await?;
        ctx.send_line(session, "  [3] SubOp").await?;
        ctx.send_line(session, "  [4] SysOp").await?;
        ctx.send(session, "Select [2]: ").await?;
        let upload_input = ctx.read_line(session).await?;
        let upload_perm = match upload_input.trim() {
            "1" => Role::Guest,
            "3" => Role::SubOp,
            "4" => Role::SysOp,
            _ => Role::Member,
        };

        // Create folder
        let mut new_folder = NewFolder::new(name)
            .with_permission(permission)
            .with_upload_perm(upload_perm);

        if !description.is_empty() {
            new_folder = new_folder.with_description(description);
        }

        match FolderRepository::create(ctx.db.conn(), &new_folder) {
            Ok(folder) => {
                let msg = ctx
                    .i18n
                    .t("admin.folder_created")
                    .replace("{{name}}", &folder.name);
                ctx.send_line(session, &msg).await?;
            }
            Err(e) => {
                ctx.send_line(session, &format!("Error: {}", e)).await?;
            }
        }

        Ok(())
    }

    /// Delete a folder.
    async fn delete_folder(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::file::FolderRepository;

        let folders = FolderRepository::list_root(ctx.db.conn())?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.delete_folder")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if folders.is_empty() {
            ctx.send_line(session, ctx.i18n.t("file.no_folders")).await?;
            ctx.send_line(session, "").await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Show folder list
        for (i, folder) in folders.iter().enumerate() {
            let file_count = FolderRepository::count_files(ctx.db.conn(), folder.id)?;
            ctx.send_line(
                session,
                &format!(
                    "  [{}] {} ({} files)",
                    i + 1,
                    folder.name,
                    file_count
                ),
            )
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
            return Ok(());
        }

        if let Some(num) = ctx.parse_number(input) {
            let idx = (num - 1) as usize;
            if idx < folders.len() {
                let folder = &folders[idx];
                let file_count = FolderRepository::count_files(ctx.db.conn(), folder.id)?;

                if file_count > 0 {
                    ctx.send_line(
                        session,
                        &ctx.i18n
                            .t("admin.folder_has_files")
                            .replace("{{count}}", &file_count.to_string()),
                    )
                    .await?;
                    ctx.send(session, &format!("{} [Y/N]: ", ctx.i18n.t("common.confirm")))
                        .await?;
                    let confirm = ctx.read_line(session).await?;
                    if !confirm.trim().eq_ignore_ascii_case("y") {
                        return Ok(());
                    }
                }

                match FolderRepository::delete(ctx.db.conn(), folder.id) {
                    Ok(true) => {
                        let msg = ctx
                            .i18n
                            .t("admin.folder_deleted")
                            .replace("{{name}}", &folder.name);
                        ctx.send_line(session, &msg).await?;
                    }
                    Ok(false) => {
                        ctx.send_line(session, ctx.i18n.t("admin.folder_not_found"))
                            .await?;
                    }
                    Err(e) => {
                        ctx.send_line(session, &format!("Error: {}", e)).await?;
                    }
                }
            }
        }

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
