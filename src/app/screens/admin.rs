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
            ctx.send_line(
                session,
                &format!("  [5] {}", ctx.i18n.t("admin.content_management")),
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
                &format!("  [6] {}", ctx.i18n.t("admin.user_list")),
            )
            .await?;
            // SysOp only: change role
            if Self::is_sysop(ctx, session) {
                ctx.send_line(
                    session,
                    &format!("  [7] {}", ctx.i18n.t("admin.change_role")),
                )
                .await?;
            }
            ctx.send_line(
                session,
                &format!("  [8] {}", ctx.i18n.t("admin.suspend_user")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [9] {}", ctx.i18n.t("admin.activate_user")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [10] {}", ctx.i18n.t("admin.session_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [11] {}", ctx.i18n.t("admin.reset_password")),
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
                &format!("  [12] {}", ctx.i18n.t("admin.chat_room_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [13] {}", ctx.i18n.t("admin.create_chat_room")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [14] {}", ctx.i18n.t("admin.delete_chat_room")),
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
                &format!("  [15] {}", ctx.i18n.t("admin.folder_list")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [16] {}", ctx.i18n.t("admin.create_folder")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [17] {}", ctx.i18n.t("admin.delete_folder")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.system_status")),
            )
            .await?;
            ctx.send_line(session, "  [18] System Status").await?;
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
                "5" => Self::manage_content(ctx, session).await?,
                "6" => Self::show_user_list(ctx, session).await?,
                "7" => Self::change_user_role(ctx, session).await?,
                "8" => Self::suspend_user(ctx, session).await?,
                "9" => Self::activate_user(ctx, session).await?,
                "10" => Self::show_sessions(ctx, session).await?,
                "11" => Self::reset_user_password(ctx, session).await?,
                "12" => Self::show_chat_rooms(ctx, session).await?,
                "13" => Self::create_chat_room(ctx, session).await?,
                "14" => Self::delete_chat_room(ctx, session).await?,
                "15" => Self::show_folders(ctx, session).await?,
                "16" => Self::create_folder(ctx, session).await?,
                "17" => Self::delete_folder(ctx, session).await?,
                "18" => Self::show_system_status(ctx, session).await?,
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

    /// Change user role (SysOp only).
    async fn change_user_role(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<()> {
        use crate::admin::{AdminError, UserAdminService};
        use crate::db::{Role, UserRepository};

        // Check SysOp permission
        if !Self::is_sysop(ctx, session) {
            ctx.send_line(session, ctx.i18n.t("admin.sysop_required"))
                .await?;
            return Ok(());
        }

        // Get current user
        let current_user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        let current_user = {
            let user_repo = UserRepository::new(&ctx.db);
            match user_repo.get_by_id(current_user_id)? {
                Some(u) => u,
                None => return Ok(()),
            }
        };

        // Get all users
        let users = {
            let user_repo = UserRepository::new(&ctx.db);
            user_repo.list_all()?
        };

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.change_role")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if users.is_empty() {
            ctx.send_line(session, "No users found.").await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Display user list
        ctx.send_line(
            session,
            &format!(
                "{:<4} {:<16} {:<16} {:<8}",
                ctx.i18n.t("common.number"),
                ctx.i18n.t("profile.username"),
                ctx.i18n.t("profile.nickname"),
                ctx.i18n.t("member.role")
            ),
        )
        .await?;
        ctx.send_line(session, &"-".repeat(50)).await?;

        for (i, user) in users.iter().enumerate() {
            let role_name = Self::role_to_string(&user.role, ctx);
            let status = if !user.is_active { " [停止]" } else { "" };
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<16} {:<16} {}{}",
                    i + 1,
                    user.username,
                    user.nickname,
                    role_name,
                    status
                ),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("admin.user_number_to_change_role"),
                ctx.i18n.t("common.cancel")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") || input.is_empty() {
            return Ok(());
        }

        let user_num: usize = match input.parse() {
            Ok(n) if n > 0 && n <= users.len() => n,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        let target_user = &users[user_num - 1];

        // Show role selection
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {} ({})",
                ctx.i18n.t("admin.select_new_role"),
                target_user.nickname,
                Self::role_to_string(&target_user.role, ctx)
            ),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("  [1] {}", ctx.i18n.t("role.guest")))
            .await?;
        ctx.send_line(session, &format!("  [2] {}", ctx.i18n.t("role.member")))
            .await?;
        ctx.send_line(session, &format!("  [3] {}", ctx.i18n.t("role.subop")))
            .await?;
        ctx.send_line(session, &format!("  [4] {}", ctx.i18n.t("role.sysop")))
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!("{} [Q={}]: ", ctx.i18n.t("common.number"), ctx.i18n.t("common.cancel")),
        )
        .await?;

        let role_input = ctx.read_line(session).await?;
        let role_input = role_input.trim();

        if role_input.eq_ignore_ascii_case("q") || role_input.is_empty() {
            return Ok(());
        }

        let new_role = match role_input {
            "1" => Role::Guest,
            "2" => Role::Member,
            "3" => Role::SubOp,
            "4" => Role::SysOp,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        // Call UserAdminService to change role
        let service = UserAdminService::new(&ctx.db);
        match service.change_user_role(target_user.id, new_role, &current_user) {
            Ok(updated) => {
                let role_name = Self::role_to_string(&updated.role, ctx);
                let msg = ctx
                    .i18n
                    .t("admin.role_changed")
                    .replace("{{name}}", &updated.nickname)
                    .replace("{{role}}", &role_name);
                ctx.send_line(session, &msg).await?;
            }
            Err(AdminError::CannotModifySelf) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_change_own_role"))
                    .await?;
            }
            Err(AdminError::LastSysOp) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_demote_last_sysop"))
                    .await?;
            }
            Err(AdminError::Permission(_)) => {
                ctx.send_line(session, ctx.i18n.t("admin.sysop_required"))
                    .await?;
            }
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("common.error"), e),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Suspend a user (ban).
    async fn suspend_user(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::admin::{AdminError, UserAdminService};
        use crate::db::UserRepository;

        // Get current admin user
        let current_user = match session.user_id() {
            Some(user_id) => {
                let user_repo = UserRepository::new(&ctx.db);
                match user_repo.get_by_id(user_id)? {
                    Some(user) => user,
                    None => {
                        ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                            .await?;
                        return Ok(());
                    }
                }
            }
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        // Get all active users
        let user_repo = UserRepository::new(&ctx.db);
        let all_users = user_repo.list_all()?;
        let users: Vec<_> = all_users.into_iter().filter(|u| u.is_active).collect();

        if users.is_empty() {
            ctx.send_line(session, ctx.i18n.t("member.no_members"))
                .await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Show user list
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.suspend_user")),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!(
                "{:<4} {:<16} {:<16} {}",
                ctx.i18n.t("common.number"),
                ctx.i18n.t("profile.username"),
                ctx.i18n.t("profile.nickname"),
                ctx.i18n.t("member.role")
            ),
        )
        .await?;
        ctx.send_line(session, &"-".repeat(50)).await?;

        for (i, user) in users.iter().enumerate() {
            let role_name = Self::role_to_string(&user.role, ctx);
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<16} {:<16} {}",
                    i + 1,
                    user.username,
                    user.nickname,
                    role_name
                ),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("admin.user_number_to_suspend"),
                ctx.i18n.t("common.cancel")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") || input.is_empty() {
            return Ok(());
        }

        let user_num: usize = match input.parse() {
            Ok(n) if n > 0 && n <= users.len() => n,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        let target_user = &users[user_num - 1];

        // Confirmation
        ctx.send_line(session, "").await?;
        let confirm_msg = ctx
            .i18n
            .t("admin.confirm_suspend")
            .replace("{{name}}", &target_user.nickname);
        ctx.send(session, &format!("{} ", confirm_msg)).await?;

        let confirm = ctx.read_line(session).await?;
        if !confirm.trim().eq_ignore_ascii_case("y") {
            return Ok(());
        }

        // Call UserAdminService to suspend user
        let service = UserAdminService::new(&ctx.db);
        match service.suspend_user(target_user.id, &current_user) {
            Ok(updated) => {
                let msg = ctx
                    .i18n
                    .t("admin.user_suspended")
                    .replace("{{name}}", &updated.nickname);
                ctx.send_line(session, &msg).await?;
            }
            Err(AdminError::CannotModifySelf) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_suspend_self"))
                    .await?;
            }
            Err(AdminError::LastSysOp) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_suspend_last_sysop"))
                    .await?;
            }
            Err(AdminError::Permission(_)) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_suspend_higher_role"))
                    .await?;
            }
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("common.error"), e),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;
        Ok(())
    }

    /// Activate a suspended user.
    async fn activate_user(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::admin::{AdminError, UserAdminService};
        use crate::db::UserRepository;

        // Get current admin user
        let current_user = match session.user_id() {
            Some(user_id) => {
                let user_repo = UserRepository::new(&ctx.db);
                match user_repo.get_by_id(user_id)? {
                    Some(user) => user,
                    None => {
                        ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                            .await?;
                        return Ok(());
                    }
                }
            }
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        // Get all suspended users
        let user_repo = UserRepository::new(&ctx.db);
        let all_users = user_repo.list_all()?;
        let users: Vec<_> = all_users.into_iter().filter(|u| !u.is_active).collect();

        if users.is_empty() {
            ctx.send_line(session, ctx.i18n.t("admin.no_suspended_users"))
                .await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Show user list
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.activate_user")),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!(
                "{:<4} {:<16} {:<16} {}",
                ctx.i18n.t("common.number"),
                ctx.i18n.t("profile.username"),
                ctx.i18n.t("profile.nickname"),
                ctx.i18n.t("member.role")
            ),
        )
        .await?;
        ctx.send_line(session, &"-".repeat(50)).await?;

        for (i, user) in users.iter().enumerate() {
            let role_name = Self::role_to_string(&user.role, ctx);
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<16} {:<16} {}",
                    i + 1,
                    user.username,
                    user.nickname,
                    role_name
                ),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("admin.user_number_to_activate"),
                ctx.i18n.t("common.cancel")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") || input.is_empty() {
            return Ok(());
        }

        let user_num: usize = match input.parse() {
            Ok(n) if n > 0 && n <= users.len() => n,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        let target_user = &users[user_num - 1];

        // Confirmation
        ctx.send_line(session, "").await?;
        let confirm_msg = ctx
            .i18n
            .t("admin.confirm_activate")
            .replace("{{name}}", &target_user.nickname);
        ctx.send(session, &format!("{} ", confirm_msg)).await?;

        let confirm = ctx.read_line(session).await?;
        if !confirm.trim().eq_ignore_ascii_case("y") {
            return Ok(());
        }

        // Call UserAdminService to activate user
        let service = UserAdminService::new(&ctx.db);
        match service.activate_user(target_user.id, &current_user) {
            Ok(updated) => {
                let msg = ctx
                    .i18n
                    .t("admin.user_activated")
                    .replace("{{name}}", &updated.nickname);
                ctx.send_line(session, &msg).await?;
            }
            Err(AdminError::Permission(_)) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_suspend_higher_role"))
                    .await?;
            }
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("common.error"), e),
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
        use crate::admin::{AdminError, SessionAdminService};
        use crate::db::UserRepository;

        // Get current admin user
        let current_user = match session.user_id() {
            Some(user_id) => {
                let user_repo = UserRepository::new(&ctx.db);
                match user_repo.get_by_id(user_id)? {
                    Some(user) => user,
                    None => {
                        ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                            .await?;
                        return Ok(());
                    }
                }
            }
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        let is_sysop = Self::is_sysop(ctx, session);

        // Create SessionAdminService
        let service = SessionAdminService::new((*ctx.session_manager).clone());

        // Get session list
        let sessions = match service.list_sessions(&current_user).await {
            Ok(s) => s,
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("common.error"), e),
                )
                .await?;
                ctx.wait_for_enter(session).await?;
                return Ok(());
            }
        };

        loop {
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.session_list")),
            )
            .await?;
            ctx.send_line(session, "").await?;

            if sessions.is_empty() {
                ctx.send_line(session, ctx.i18n.t("admin.no_sessions"))
                    .await?;
                ctx.send_line(session, "").await?;
                ctx.wait_for_enter(session).await?;
                return Ok(());
            }

            // Display session list
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<16} {:<16} {:<10}",
                    ctx.i18n.t("common.number"),
                    ctx.i18n.t("common.user"),
                    "IP",
                    ctx.i18n.t("common.current")
                ),
            )
            .await?;
            ctx.send_line(session, &"-".repeat(50)).await?;

            for (i, sess) in sessions.iter().enumerate() {
                let username = sess.username.as_deref().unwrap_or("(Guest)");
                let ip = sess.peer_addr.ip().to_string();
                let state = Self::session_state_to_string(&sess.state, ctx);
                let is_self = sess.id == session.id();
                let marker = if is_self { " *" } else { "" };

                ctx.send_line(
                    session,
                    &format!(
                        "{:<4} {:<16} {:<16} {}{}",
                        i + 1,
                        username,
                        ip,
                        state,
                        marker
                    ),
                )
                .await?;
            }

            ctx.send_line(session, "").await?;

            // SysOp can disconnect users
            if is_sysop && sessions.len() > 1 {
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("admin.session_number_to_disconnect"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;

                let input = ctx.read_line(session).await?;
                let input = input.trim();

                if input.eq_ignore_ascii_case("q") || input.is_empty() {
                    return Ok(());
                }

                let sess_num: usize = match input.parse() {
                    Ok(n) if n > 0 && n <= sessions.len() => n,
                    _ => {
                        ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                            .await?;
                        continue;
                    }
                };

                let target_session = &sessions[sess_num - 1];

                // Confirmation
                ctx.send_line(session, "").await?;
                let target_name = target_session
                    .username
                    .as_deref()
                    .unwrap_or("Guest");
                let confirm_msg = ctx
                    .i18n
                    .t("admin.confirm_disconnect")
                    .replace("{{name}}", target_name);
                ctx.send(session, &format!("{} [Y/N]: ", confirm_msg)).await?;

                let confirm = ctx.read_line(session).await?;
                if !confirm.trim().eq_ignore_ascii_case("y") {
                    continue;
                }

                // Force disconnect
                match service.force_disconnect(target_session.id, &current_user).await {
                    Ok(_) => {
                        let msg = ctx
                            .i18n
                            .t("admin.session_disconnected")
                            .replace("{{name}}", target_name);
                        ctx.send_line(session, &msg).await?;
                    }
                    Err(AdminError::CannotModifySelf) => {
                        ctx.send_line(session, ctx.i18n.t("admin.cannot_disconnect_self"))
                            .await?;
                    }
                    Err(e) => {
                        ctx.send_line(
                            session,
                            &format!("{}: {}", ctx.i18n.t("common.error"), e),
                        )
                        .await?;
                    }
                }

                ctx.send_line(session, "").await?;
                ctx.wait_for_enter(session).await?;
                return Ok(());
            } else {
                ctx.wait_for_enter(session).await?;
                return Ok(());
            }
        }
    }

    /// Convert session state to localized string.
    fn session_state_to_string(state: &crate::server::SessionState, ctx: &ScreenContext) -> String {
        use crate::server::SessionState;
        match state {
            SessionState::Welcome => ctx.i18n.t("admin.session_state_welcome").to_string(),
            SessionState::Login => ctx.i18n.t("admin.session_state_login").to_string(),
            SessionState::Registration => ctx.i18n.t("admin.session_state_registration").to_string(),
            SessionState::MainMenu => ctx.i18n.t("admin.session_state_mainmenu").to_string(),
            SessionState::Board => ctx.i18n.t("admin.session_state_board").to_string(),
            SessionState::Chat => ctx.i18n.t("admin.session_state_chat").to_string(),
            SessionState::Mail => ctx.i18n.t("admin.session_state_mail").to_string(),
            SessionState::Files => ctx.i18n.t("admin.session_state_files").to_string(),
            SessionState::Admin => ctx.i18n.t("admin.session_state_admin").to_string(),
            SessionState::Closing => ctx.i18n.t("admin.session_state_closing").to_string(),
        }
    }

    /// Reset a user's password.
    async fn reset_user_password(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<()> {
        use crate::admin::{AdminError, UserAdminService};
        use crate::db::UserRepository;

        // Get current admin user
        let current_user = match session.user_id() {
            Some(user_id) => {
                let user_repo = UserRepository::new(&ctx.db);
                match user_repo.get_by_id(user_id)? {
                    Some(user) => user,
                    None => {
                        ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                            .await?;
                        return Ok(());
                    }
                }
            }
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        // Get all users
        let user_repo = UserRepository::new(&ctx.db);
        let users = user_repo.list_all()?;

        if users.is_empty() {
            ctx.send_line(session, ctx.i18n.t("member.no_members"))
                .await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Show user list
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("admin.reset_password")),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!(
                "{:<4} {:<16} {:<16} {}",
                ctx.i18n.t("common.number"),
                ctx.i18n.t("profile.username"),
                ctx.i18n.t("profile.nickname"),
                ctx.i18n.t("member.role")
            ),
        )
        .await?;
        ctx.send_line(session, &"-".repeat(50)).await?;

        for (i, user) in users.iter().enumerate() {
            let role_name = Self::role_to_string(&user.role, ctx);
            let status = if !user.is_active { " [停止]" } else { "" };
            ctx.send_line(
                session,
                &format!(
                    "{:<4} {:<16} {:<16} {}{}",
                    i + 1,
                    user.username,
                    user.nickname,
                    role_name,
                    status
                ),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("admin.user_number_to_reset_password"),
                ctx.i18n.t("common.cancel")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("q") || input.is_empty() {
            return Ok(());
        }

        let user_num: usize = match input.parse() {
            Ok(n) if n > 0 && n <= users.len() => n,
            _ => {
                ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                    .await?;
                return Ok(());
            }
        };

        let target_user = &users[user_num - 1];

        // Confirmation
        ctx.send_line(session, "").await?;
        let confirm_msg = ctx
            .i18n
            .t("admin.confirm_reset_password")
            .replace("{{name}}", &target_user.nickname);
        ctx.send(session, &format!("{} ", confirm_msg)).await?;

        let confirm = ctx.read_line(session).await?;
        if !confirm.trim().eq_ignore_ascii_case("y") {
            return Ok(());
        }

        // Call UserAdminService to reset password
        let service = UserAdminService::new(&ctx.db);
        match service.reset_user_password(target_user.id, &current_user) {
            Ok(new_password) => {
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("admin.password_reset_success"))
                    .await?;
                ctx.send_line(session, "").await?;
                let password_msg = ctx
                    .i18n
                    .t("admin.new_password_is")
                    .replace("{{password}}", &new_password);
                ctx.send_line(session, &password_msg).await?;
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("admin.password_reset_note"))
                    .await?;
            }
            Err(AdminError::Permission(_)) => {
                ctx.send_line(session, ctx.i18n.t("admin.cannot_suspend_higher_role"))
                    .await?;
            }
            Err(e) => {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("common.error"), e),
                )
                .await?;
            }
        }

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

    /// Content management - hierarchical navigation for posts and threads.
    async fn manage_content(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        use crate::board::{BoardRepository, BoardType, PostRepository, ThreadRepository};
        use crate::db::UserRepository;

        // Get current admin user
        let current_user = match session.user_id() {
            Some(user_id) => {
                let user_repo = UserRepository::new(&ctx.db);
                match user_repo.get_by_id(user_id)? {
                    Some(user) => user,
                    None => {
                        ctx.send_line(session, ctx.i18n.t("error.user_not_found"))
                            .await?;
                        return Ok(());
                    }
                }
            }
            None => {
                ctx.send_line(session, ctx.i18n.t("error.not_logged_in"))
                    .await?;
                return Ok(());
            }
        };

        // Level 1: Board list
        loop {
            let board_repo = BoardRepository::new(&ctx.db);
            let boards = board_repo.list_all()?;

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("admin.content_management")),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send_line(session, ctx.i18n.t("admin.select_board")).await?;
            ctx.send_line(session, "").await?;

            if boards.is_empty() {
                ctx.send_line(session, ctx.i18n.t("board.no_boards"))
                    .await?;
                ctx.wait_for_enter(session).await?;
                return Ok(());
            }

            for (i, board) in boards.iter().enumerate() {
                let count_info = match board.board_type {
                    BoardType::Thread => {
                        let thread_repo = ThreadRepository::new(&ctx.db);
                        let thread_count = thread_repo.count_by_board(board.id).unwrap_or(0);
                        format!("{} threads", thread_count)
                    }
                    BoardType::Flat => {
                        let post_repo = PostRepository::new(&ctx.db);
                        let post_count = post_repo.count_by_flat_board(board.id).unwrap_or(0);
                        format!("{} posts", post_count)
                    }
                };
                let type_marker = match board.board_type {
                    BoardType::Thread => "[T]",
                    BoardType::Flat => "[F]",
                };
                ctx.send_line(
                    session,
                    &format!(
                        "  [{}] {} {} ({})",
                        i + 1,
                        type_marker,
                        board.name,
                        count_info
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

            let board_num: usize = match input.parse() {
                Ok(n) if n > 0 && n <= boards.len() => n,
                _ => {
                    ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                        .await?;
                    continue;
                }
            };

            let board = &boards[board_num - 1];

            // Branch based on board type
            match board.board_type {
                BoardType::Flat => {
                    // Flat board: directly show posts
                    Self::manage_flat_board_content(ctx, session, board, &current_user).await?;
                }
                BoardType::Thread => {
                    // Thread board: show threads first
                    Self::manage_thread_board_content(ctx, session, board, &current_user).await?;
                }
            }
        }
    }

    /// Manage content for flat-type boards (posts only, no threads).
    async fn manage_flat_board_content(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board: &crate::board::Board,
        current_user: &crate::db::User,
    ) -> Result<()> {
        use crate::admin::{AdminError, ContentAdminService, PostDeletionMode};
        use crate::board::PostRepository;

        loop {
            let post_repo = PostRepository::new(&ctx.db);
            let posts = post_repo.list_by_flat_board(board.id)?;

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!(
                    "=== {} - {} ===",
                    ctx.i18n.t("admin.content_management"),
                    board.name
                ),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send_line(session, ctx.i18n.t("admin.select_post")).await?;
            ctx.send_line(session, "").await?;

            if posts.is_empty() {
                ctx.send_line(session, ctx.i18n.t("admin.no_posts_in_thread"))
                    .await?;
                ctx.send_line(session, "").await?;
                ctx.send_line(
                    session,
                    &format!("[Q] {}", ctx.i18n.t("admin.back_to_boards")),
                )
                .await?;
                ctx.send(session, &format!("{}: ", ctx.i18n.t("menu.select_prompt")))
                    .await?;
                let _ = ctx.read_line(session).await?;
                return Ok(());
            }

            for (i, post) in posts.iter().enumerate() {
                let preview = if post.body.chars().count() > 40 {
                    format!("{}...", post.body.chars().take(37).collect::<String>())
                } else {
                    post.body.clone()
                };
                let preview = preview.replace('\n', " ").replace('\r', "");
                ctx.send_line(
                    session,
                    &format!("  [{}] #{}: {}", i + 1, post.id, preview),
                )
                .await?;
            }

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("[D] {}", ctx.i18n.t("admin.delete_post")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("[Q] {}", ctx.i18n.t("admin.back_to_boards")),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send(session, &format!("{}: ", ctx.i18n.t("menu.select_prompt")))
                .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") || input.is_empty() {
                return Ok(());
            }

            if input.eq_ignore_ascii_case("d") {
                // Delete post mode
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("admin.post_number_to_delete"),
                        ctx.i18n.t("common.cancel")
                    ),
                )
                .await?;

                let del_input = ctx.read_line(session).await?;
                let del_input = del_input.trim();

                if del_input.eq_ignore_ascii_case("q") || del_input.is_empty() {
                    continue;
                }

                let post_num: usize = match del_input.parse() {
                    Ok(n) if n > 0 && n <= posts.len() => n,
                    _ => {
                        ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                            .await?;
                        continue;
                    }
                };

                let target_post = &posts[post_num - 1];

                // Select deletion mode
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("admin.select_deletion_mode"))
                    .await?;
                ctx.send_line(
                    session,
                    &format!("  [1] {}", ctx.i18n.t("admin.soft_delete")),
                )
                .await?;
                ctx.send_line(
                    session,
                    &format!("  [2] {}", ctx.i18n.t("admin.hard_delete")),
                )
                .await?;
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("menu.select_prompt"),
                        ctx.i18n.t("common.cancel")
                    ),
                )
                .await?;

                let mode_input = ctx.read_line(session).await?;
                let mode_input = mode_input.trim();

                if mode_input.eq_ignore_ascii_case("q") || mode_input.is_empty() {
                    continue;
                }

                let mode = match mode_input {
                    "1" => PostDeletionMode::Soft,
                    "2" => PostDeletionMode::Hard,
                    _ => {
                        ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                            .await?;
                        continue;
                    }
                };

                // Confirmation
                let confirm_msg = ctx
                    .i18n
                    .t("admin.confirm_delete_post")
                    .replace("{{id}}", &target_post.id.to_string());
                ctx.send(session, &format!("{} ", confirm_msg)).await?;

                let confirm = ctx.read_line(session).await?;
                if !confirm.trim().eq_ignore_ascii_case("y") {
                    continue;
                }

                let service = ContentAdminService::new(&ctx.db);
                match service.delete_post(target_post.id, mode, current_user) {
                    Ok(true) => {
                        let msg = ctx
                            .i18n
                            .t("admin.post_deleted")
                            .replace("{{id}}", &target_post.id.to_string());
                        ctx.send_line(session, &msg).await?;
                    }
                    Ok(false) => {
                        ctx.send_line(session, ctx.i18n.t("admin.post_not_found"))
                            .await?;
                    }
                    Err(AdminError::NotFound(_)) => {
                        ctx.send_line(session, ctx.i18n.t("admin.post_not_found"))
                            .await?;
                    }
                    Err(e) => {
                        ctx.send_line(
                            session,
                            &format!("{}: {}", ctx.i18n.t("common.error"), e),
                        )
                        .await?;
                    }
                }
                continue;
            }

            // Invalid input
            ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                .await?;
        }
    }

    /// Manage content for thread-type boards.
    async fn manage_thread_board_content(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board: &crate::board::Board,
        current_user: &crate::db::User,
    ) -> Result<()> {
        use crate::admin::{AdminError, ContentAdminService};
        use crate::board::{PostRepository, ThreadRepository};

        // Level 2: Thread list for selected board
        loop {
            let thread_repo = ThreadRepository::new(&ctx.db);
            let threads = thread_repo.list_by_board(board.id)?;

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!(
                    "=== {} - {} ===",
                    ctx.i18n.t("admin.content_management"),
                    board.name
                ),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send_line(session, ctx.i18n.t("admin.select_thread"))
                .await?;
            ctx.send_line(session, "").await?;

            if threads.is_empty() {
                ctx.send_line(session, ctx.i18n.t("admin.no_threads_in_board"))
                    .await?;
                ctx.send_line(session, "").await?;
                ctx.send_line(
                    session,
                    &format!("[Q] {}", ctx.i18n.t("admin.back_to_boards")),
                )
                .await?;
                ctx.send(
                    session,
                    &format!("{}: ", ctx.i18n.t("menu.select_prompt")),
                )
                .await?;
                let _ = ctx.read_line(session).await?;
                return Ok(());
            }

            for (i, thread) in threads.iter().enumerate() {
                let post_repo = PostRepository::new(&ctx.db);
                let post_count = post_repo.count_by_thread(thread.id).unwrap_or(0);
                ctx.send_line(
                    session,
                    &format!(
                        "  [{}] {} ({} posts)",
                        i + 1,
                        if thread.title.chars().count() > 30 {
                            format!(
                                "{}...",
                                thread.title.chars().take(27).collect::<String>()
                            )
                        } else {
                            thread.title.clone()
                        },
                        post_count
                    ),
                )
                .await?;
            }

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("[D] {}", ctx.i18n.t("admin.delete_thread")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("[Q] {}", ctx.i18n.t("admin.back_to_boards")),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send(
                session,
                &format!("{}: ", ctx.i18n.t("menu.select_prompt")),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") || input.is_empty() {
                return Ok(());
            }

            if input.eq_ignore_ascii_case("d") {
                // Delete thread mode
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("admin.thread_number_to_delete"),
                        ctx.i18n.t("common.cancel")
                    ),
                )
                .await?;

                let del_input = ctx.read_line(session).await?;
                let del_input = del_input.trim();

                if del_input.eq_ignore_ascii_case("q") || del_input.is_empty() {
                    continue;
                }

                let thread_num: usize = match del_input.parse() {
                    Ok(n) if n > 0 && n <= threads.len() => n,
                    _ => {
                        ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                            .await?;
                        continue;
                    }
                };

                let target_thread = &threads[thread_num - 1];
                let post_repo = PostRepository::new(&ctx.db);
                let post_count = post_repo.count_by_thread(target_thread.id).unwrap_or(0);

                let confirm_msg = ctx
                    .i18n
                    .t("admin.confirm_delete_thread")
                    .replace("{{title}}", &target_thread.title)
                    .replace("{{posts}}", &post_count.to_string());
                ctx.send(session, &format!("{} ", confirm_msg)).await?;

                let confirm = ctx.read_line(session).await?;
                if !confirm.trim().eq_ignore_ascii_case("y") {
                    continue;
                }

                let service = ContentAdminService::new(&ctx.db);
                match service.delete_thread(target_thread.id, current_user) {
                    Ok(true) => {
                        let msg = ctx
                            .i18n
                            .t("admin.thread_deleted")
                            .replace("{{title}}", &target_thread.title);
                        ctx.send_line(session, &msg).await?;
                    }
                    Ok(false) => {
                        ctx.send_line(session, ctx.i18n.t("admin.thread_not_found"))
                            .await?;
                    }
                    Err(AdminError::NotFound(_)) => {
                        ctx.send_line(session, ctx.i18n.t("admin.thread_not_found"))
                            .await?;
                    }
                    Err(e) => {
                        ctx.send_line(
                            session,
                            &format!("{}: {}", ctx.i18n.t("common.error"), e),
                        )
                        .await?;
                    }
                }
                continue;
            }

            // Select thread to view posts
            let thread_num: usize = match input.parse() {
                Ok(n) if n > 0 && n <= threads.len() => n,
                _ => {
                    ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                        .await?;
                    continue;
                }
            };

            let thread = &threads[thread_num - 1];

            // Level 3: Post list for selected thread
            Self::manage_thread_posts(ctx, session, thread, current_user).await?;
        }
    }

    /// Manage posts within a thread.
    async fn manage_thread_posts(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        thread: &crate::board::Thread,
        current_user: &crate::db::User,
    ) -> Result<()> {
        use crate::admin::{AdminError, ContentAdminService, PostDeletionMode};
        use crate::board::PostRepository;

        loop {
            let post_repo = PostRepository::new(&ctx.db);
            let posts = post_repo.list_by_thread(thread.id)?;

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!(
                    "=== {} - {} ===",
                    ctx.i18n.t("admin.content_management"),
                    thread.title
                ),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send_line(session, ctx.i18n.t("admin.select_post"))
                .await?;
            ctx.send_line(session, "").await?;

            if posts.is_empty() {
                ctx.send_line(session, ctx.i18n.t("admin.no_posts_in_thread"))
                    .await?;
                ctx.send_line(session, "").await?;
                ctx.send_line(
                    session,
                    &format!("[Q] {}", ctx.i18n.t("admin.back_to_threads")),
                )
                .await?;
                ctx.send(
                    session,
                    &format!("{}: ", ctx.i18n.t("menu.select_prompt")),
                )
                .await?;
                let _ = ctx.read_line(session).await?;
                return Ok(());
            }

            for (i, post) in posts.iter().enumerate() {
                let preview = if post.body.chars().count() > 40 {
                    format!("{}...", post.body.chars().take(37).collect::<String>())
                } else {
                    post.body.clone()
                };
                let preview = preview.replace('\n', " ").replace('\r', "");
                ctx.send_line(
                    session,
                    &format!("  [{}] #{}: {}", i + 1, post.id, preview),
                )
                .await?;
            }

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("[D] {}", ctx.i18n.t("admin.delete_post")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("[Q] {}", ctx.i18n.t("admin.back_to_threads")),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send(
                session,
                &format!("{}: ", ctx.i18n.t("menu.select_prompt")),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") || input.is_empty() {
                return Ok(());
            }

            if input.eq_ignore_ascii_case("d") {
                // Delete post mode
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("admin.post_number_to_delete"),
                        ctx.i18n.t("common.cancel")
                    ),
                )
                .await?;

                let del_input = ctx.read_line(session).await?;
                let del_input = del_input.trim();

                if del_input.eq_ignore_ascii_case("q") || del_input.is_empty() {
                    continue;
                }

                let post_num: usize = match del_input.parse() {
                    Ok(n) if n > 0 && n <= posts.len() => n,
                    _ => {
                        ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                            .await?;
                        continue;
                    }
                };

                let target_post = &posts[post_num - 1];

                // Select deletion mode
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("admin.select_deletion_mode"))
                    .await?;
                ctx.send_line(
                    session,
                    &format!("  [1] {}", ctx.i18n.t("admin.soft_delete")),
                )
                .await?;
                ctx.send_line(
                    session,
                    &format!("  [2] {}", ctx.i18n.t("admin.hard_delete")),
                )
                .await?;
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("menu.select_prompt"),
                        ctx.i18n.t("common.cancel")
                    ),
                )
                .await?;

                let mode_input = ctx.read_line(session).await?;
                let mode_input = mode_input.trim();

                if mode_input.eq_ignore_ascii_case("q") || mode_input.is_empty() {
                    continue;
                }

                let mode = match mode_input {
                    "1" => PostDeletionMode::Soft,
                    "2" => PostDeletionMode::Hard,
                    _ => {
                        ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                            .await?;
                        continue;
                    }
                };

                // Confirmation
                let confirm_msg = ctx
                    .i18n
                    .t("admin.confirm_delete_post")
                    .replace("{{id}}", &target_post.id.to_string());
                ctx.send(session, &format!("{} ", confirm_msg)).await?;

                let confirm = ctx.read_line(session).await?;
                if !confirm.trim().eq_ignore_ascii_case("y") {
                    continue;
                }

                let service = ContentAdminService::new(&ctx.db);
                match service.delete_post(target_post.id, mode, current_user) {
                    Ok(true) => {
                        let msg = ctx
                            .i18n
                            .t("admin.post_deleted")
                            .replace("{{id}}", &target_post.id.to_string());
                        ctx.send_line(session, &msg).await?;
                    }
                    Ok(false) => {
                        ctx.send_line(session, ctx.i18n.t("admin.post_not_found"))
                            .await?;
                    }
                    Err(AdminError::NotFound(_)) => {
                        ctx.send_line(session, ctx.i18n.t("admin.post_not_found"))
                            .await?;
                    }
                    Err(e) => {
                        ctx.send_line(
                            session,
                            &format!("{}: {}", ctx.i18n.t("common.error"), e),
                        )
                        .await?;
                    }
                }
                continue;
            }

            // Invalid input
            ctx.send_line(session, ctx.i18n.t("common.invalid_input"))
                .await?;
        }
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

    fn is_sysop(ctx: &ScreenContext, session: &TelnetSession) -> bool {
        use crate::db::{Role, UserRepository};

        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(&ctx.db);
            if let Ok(Some(user)) = user_repo.get_by_id(user_id) {
                return user.role == Role::SysOp;
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
