//! Board screen handler.

use tracing::error;

use super::common::{Pagination, ScreenContext};
use crate::datetime::format_datetime;
use super::ScreenResult;
use crate::board::{
    BoardRepository, BoardService, BoardType, Pagination as BoardPagination, PostRepository,
    ThreadRepository, UnreadPostWithBoard, UnreadRepository,
};
use crate::db::{Role, UserRepository};
use crate::error::Result;
use crate::rate_limit::RateLimitResult;
use crate::server::{convert_caret_escape, TelnetSession};
use crate::template::Value;

/// Board screen handler.
pub struct BoardScreen;

impl BoardScreen {
    /// Run the board list screen.
    pub async fn run_list(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<ScreenResult> {
        loop {
            // Get boards
            let user_role = Self::get_user_role(ctx, session).await;
            let board_service = BoardService::new(&ctx.db);
            let boards = board_service.list_boards(user_role).await?;

            // Get unread counts for logged-in users
            let unread_counts: std::collections::HashMap<i64, i64> =
                if let Some(user_id) = session.user_id() {
                    let unread_repo = UnreadRepository::new(ctx.db.pool());
                    unread_repo
                        .get_all_unread_counts(user_id)
                        .await?
                        .into_iter()
                        .collect()
                } else {
                    std::collections::HashMap::new()
                };

            // Display board list using template
            let logged_in = session.user_id().is_some();
            let mut context = ctx.create_context();
            context.set("logged_in", Value::bool(logged_in));
            context.set("has_boards", Value::bool(!boards.is_empty()));

            let mut board_list = Vec::new();
            for (i, board) in boards.iter().enumerate() {
                let count = if board.board_type == BoardType::Thread {
                    let thread_repo = ThreadRepository::new(ctx.db.pool());
                    thread_repo.count_by_board(board.id).await?
                } else {
                    let post_repo = PostRepository::new(ctx.db.pool());
                    post_repo.count_by_flat_board(board.id).await?
                };

                let mut entry = std::collections::HashMap::new();
                entry.insert("number".to_string(), Value::string((i + 1).to_string()));
                entry.insert("name".to_string(), Value::string(&board.name));
                entry.insert("count".to_string(), Value::string(count.to_string()));

                if logged_in {
                    let unread = unread_counts.get(&board.id).copied().unwrap_or(0);
                    let unread_display = if unread > 0 {
                        format!("[{}]", unread)
                    } else {
                        String::new()
                    };
                    entry.insert("unread_display".to_string(), Value::string(unread_display));
                }

                board_list.push(Value::Object(entry));
            }
            context.set("boards", Value::List(board_list));

            let content = ctx.render_template("board/list", &context)?;
            ctx.send(session, &content).await?;

            // Prompt - show [U] option only for logged-in users
            if session.user_id().is_some() {
                ctx.send(
                    session,
                    &format!(
                        "{} [U]={} [Q={}]: ",
                        ctx.i18n.t("menu.select_prompt"),
                        ctx.i18n.t("board.read_all_unread"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;
            } else {
                ctx.send(
                    session,
                    &format!(
                        "{} [Q={}]: ",
                        ctx.i18n.t("menu.select_prompt"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;
            }

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "u" => {
                    if session.user_id().is_some() {
                        Self::run_all_unread_batch_read(ctx, session).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        let idx = (num - 1) as usize;
                        if idx < boards.len() {
                            let board = &boards[idx];
                            match board.board_type {
                                BoardType::Thread => {
                                    Self::run_thread_list(ctx, session, board.id).await?;
                                }
                                BoardType::Flat => {
                                    Self::run_flat_list(ctx, session, board.id).await?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Run the thread list screen (for thread-type boards).
    async fn run_thread_list(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board_id: i64,
    ) -> Result<ScreenResult> {
        let per_page: i64 = 10;
        let mut pagination = Pagination::new(1, per_page as usize, 0);

        loop {
            // Get board info
            let user_role = Self::get_user_role(ctx, session).await;
            let board_service = BoardService::new(&ctx.db);
            let board = board_service.get_board(board_id, user_role).await?;

            // Get threads using service with pagination
            let board_pagination =
                BoardPagination::new(pagination.offset() as i64, pagination.per_page as i64);
            let result = board_service.list_threads(board_id, user_role, board_pagination).await?;

            pagination.total = result.total as usize;

            // Display thread list using template
            let mut context = ctx.create_context();
            context.set("board_name", Value::string(board.name.clone()));
            context.set("has_threads", Value::bool(!result.items.is_empty()));

            if !result.items.is_empty() {
                // Get unread thread IDs for logged-in users
                let unread_thread_ids = if let Some(user_id) = session.user_id() {
                    let thread_ids: Vec<i64> = result.items.iter().map(|t| t.id).collect();
                    let unread_repo = UnreadRepository::new(ctx.db.pool());
                    unread_repo.get_unread_thread_ids(user_id, board_id, &thread_ids).await?
                } else {
                    std::collections::HashSet::new()
                };

                let mut thread_list = Vec::new();
                for (i, thread) in result.items.iter().enumerate() {
                    let num = pagination.offset() + i + 1;

                    let unread_mark = if unread_thread_ids.contains(&thread.id) {
                        "*"
                    } else {
                        " "
                    };

                    let mut entry = std::collections::HashMap::new();
                    entry.insert("unread_mark".to_string(), Value::string(unread_mark));
                    entry.insert("number".to_string(), Value::string(num.to_string()));
                    entry.insert("title".to_string(), Value::string(&thread.title));
                    entry.insert("post_count".to_string(), Value::string(thread.post_count.to_string()));
                    thread_list.push(Value::Object(entry));
                }
                context.set("threads", Value::List(thread_list));
            }

            context.set(
                "page_info",
                Value::string(ctx.i18n.t_with(
                    "board.page_of",
                    &[
                        ("current", &pagination.page.to_string()),
                        ("total", &pagination.total_pages().to_string()),
                    ],
                ).to_string()),
            );

            let content = ctx.render_template("board/thread_list", &context)?;
            ctx.send(session, &content).await?;

            // Prompt - show [U] and [A] options only for logged-in users
            if session.user_id().is_some() {
                ctx.send(
                    session,
                    &format!(
                        "[N]={} [P]={} [U]={} [A]={} [W]={} [Q]={}: ",
                        ctx.i18n.t("common.next"),
                        ctx.i18n.t("common.previous"),
                        ctx.i18n.t("board.read_unread"),
                        ctx.i18n.t("board.mark_all_read"),
                        ctx.i18n.t("board.new_thread"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;
            } else {
                ctx.send(
                    session,
                    &format!(
                        "[N]={} [P]={} [W]={} [Q]={}: ",
                        ctx.i18n.t("common.next"),
                        ctx.i18n.t("common.previous"),
                        ctx.i18n.t("board.new_thread"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;
            }

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "n" => pagination.next(),
                "p" => pagination.prev(),
                "u" => {
                    if session.user_id().is_some() {
                        Self::run_unread_batch_read(ctx, session, board_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                "a" => {
                    if session.user_id().is_some() {
                        Self::mark_all_as_read_for_board(ctx, session, board_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                "w" => {
                    if session.user_id().is_some() {
                        Self::create_thread(ctx, session, board_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        let offset = pagination.offset();
                        let idx = num as i64 - 1 - offset as i64;
                        if idx >= 0 && (idx as usize) < result.items.len() {
                            Self::run_thread_view(ctx, session, result.items[idx as usize].id)
                                .await?;
                        }
                    }
                }
            }
        }
    }

    /// Run the flat post list screen (for flat-type boards).
    async fn run_flat_list(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board_id: i64,
    ) -> Result<ScreenResult> {
        let per_page: i64 = 10;
        let mut pagination = Pagination::new(1, per_page as usize, 0);

        loop {
            // Get board info
            let user_role = Self::get_user_role(ctx, session).await;
            let board_service = BoardService::new(&ctx.db);
            let board = board_service.get_board(board_id, user_role).await?;

            // Get posts using service with pagination
            let board_pagination =
                BoardPagination::new(pagination.offset() as i64, pagination.per_page as i64);
            let result =
                board_service.list_posts_in_flat_board(board_id, user_role, board_pagination).await?;

            pagination.total = result.total as usize;

            // Display post list using template
            let mut context = ctx.create_context();
            context.set("board_name", Value::string(board.name.clone()));
            context.set("has_posts", Value::bool(!result.items.is_empty()));

            if !result.items.is_empty() {
                // Get last read post ID for logged-in users
                let last_read_post_id = if let Some(user_id) = session.user_id() {
                    let unread_repo = UnreadRepository::new(ctx.db.pool());
                    unread_repo.get_last_read_post_id(user_id, board_id).await?
                } else {
                    i64::MAX // For guests, mark nothing as unread
                };

                let user_repo = UserRepository::new(ctx.db.pool());
                let mut post_list = Vec::new();
                for (i, post) in result.items.iter().enumerate() {
                    let num = result.total as usize - pagination.offset() - i;
                    let title = post.title.as_deref().unwrap_or("(no title)");
                    let author = user_repo
                        .get_by_id(post.author_id)
                        .await?
                        .map(|u| u.nickname)
                        .unwrap_or_else(|| "Unknown".to_string());

                    let unread_mark = if post.id > last_read_post_id {
                        "*"
                    } else {
                        " "
                    };

                    let mut entry = std::collections::HashMap::new();
                    entry.insert("unread_mark".to_string(), Value::string(unread_mark));
                    entry.insert("number".to_string(), Value::string(num.to_string()));
                    entry.insert("title".to_string(), Value::string(title));
                    entry.insert("author".to_string(), Value::string(author));
                    post_list.push(Value::Object(entry));
                }
                context.set("posts", Value::List(post_list));
            }

            context.set(
                "page_info",
                Value::string(ctx.i18n.t_with(
                    "board.page_of",
                    &[
                        ("current", &pagination.page.to_string()),
                        ("total", &pagination.total_pages().to_string()),
                    ],
                ).to_string()),
            );

            let content = ctx.render_template("board/flat_list", &context)?;
            ctx.send(session, &content).await?;

            // Prompt - show [U] and [A] options only for logged-in users
            if session.user_id().is_some() {
                ctx.send(
                    session,
                    &format!(
                        "[N]={} [P]={} [U]={} [A]={} [W]={} [Q]={}: ",
                        ctx.i18n.t("common.next"),
                        ctx.i18n.t("common.previous"),
                        ctx.i18n.t("board.read_unread"),
                        ctx.i18n.t("board.mark_all_read"),
                        ctx.i18n.t("board.new_post"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;
            } else {
                ctx.send(
                    session,
                    &format!(
                        "[N]={} [P]={} [W]={} [Q]={}: ",
                        ctx.i18n.t("common.next"),
                        ctx.i18n.t("common.previous"),
                        ctx.i18n.t("board.new_post"),
                        ctx.i18n.t("common.back")
                    ),
                )
                .await?;
            }

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "n" => pagination.next(),
                "p" => pagination.prev(),
                "u" => {
                    if session.user_id().is_some() {
                        Self::run_unread_batch_read(ctx, session, board_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                "a" => {
                    if session.user_id().is_some() {
                        Self::mark_all_as_read_for_board(ctx, session, board_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                "w" => {
                    if session.user_id().is_some() {
                        Self::create_flat_post(ctx, session, board_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        // Convert descending number to index
                        // num = total - offset - idx, so idx = total - offset - num
                        let idx = result.total as i64 - pagination.offset() as i64 - num as i64;
                        if idx >= 0 && (idx as usize) < result.items.len() {
                            Self::run_post_view(ctx, session, result.items[idx as usize].id)
                                .await?;
                        }
                    }
                }
            }
        }
    }

    /// View a thread and its posts.
    async fn run_thread_view(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        thread_id: i64,
    ) -> Result<ScreenResult> {
        let per_page: i64 = 10;
        let mut pagination = Pagination::new(1, per_page as usize, 0);

        loop {
            // Get thread info
            let user_role = Self::get_user_role(ctx, session).await;

            // Get thread and board info, then check disable_paging
            let (thread, disable_paging) = {
                let board_service = BoardService::new(&ctx.db);
                let thread = board_service.get_thread(thread_id, user_role).await?;
                let board = board_service.get_board(thread.board_id, user_role).await?;
                (thread, board.disable_paging)
            };
            let original_paging = ctx.auto_paging_enabled();
            if disable_paging {
                ctx.set_auto_paging(false);
            }

            // Get posts using service with pagination
            let board_pagination =
                BoardPagination::new(pagination.offset() as i64, pagination.per_page as i64);
            let board_service = BoardService::new(&ctx.db);
            let result =
                board_service.list_posts_in_thread(thread_id, user_role, board_pagination).await?;

            pagination.total = result.total as usize;

            // Display thread using template
            let mut context = ctx.create_context();
            context.set("thread_title", Value::string(thread.title.clone()));
            context.set("has_posts", Value::bool(!result.items.is_empty()));

            if !result.items.is_empty() {
                let user_repo = UserRepository::new(ctx.db.pool());
                let mut post_list = Vec::new();
                for post in &result.items {
                    let author = user_repo
                        .get_by_id(post.author_id)
                        .await?
                        .map(|u| u.nickname)
                        .unwrap_or_else(|| "Unknown".to_string());

                    let formatted_time = format_datetime(
                        &post.created_at,
                        &ctx.config.server.timezone,
                        "%Y-%m-%d %H:%M",
                    );

                    let mut entry = std::collections::HashMap::new();
                    entry.insert("author".to_string(), Value::string(author));
                    entry.insert("created_at".to_string(), Value::string(formatted_time));
                    entry.insert("body".to_string(), Value::string(convert_caret_escape(&post.body)));
                    post_list.push(Value::Object(entry));
                }
                context.set("posts", Value::List(post_list));

                // Mark the last displayed post as read for logged-in users
                if let Some(user_id) = session.user_id() {
                    if let Some(last_post) = result.items.last() {
                        let unread_repo = UnreadRepository::new(ctx.db.pool());
                        unread_repo.mark_as_read(user_id, thread.board_id, last_post.id).await?;
                    }
                }
            }

            context.set(
                "page_info",
                Value::string(ctx.i18n.t_with(
                    "board.page_of",
                    &[
                        ("current", &pagination.page.to_string()),
                        ("total", &pagination.total_pages().to_string()),
                    ],
                ).to_string()),
            );

            let content = ctx.render_template("board/post_view", &context)?;
            ctx.send(session, &content).await?;

            // Restore auto-paging setting before prompt
            ctx.set_auto_paging(original_paging);

            // Prompt
            ctx.send(
                session,
                &format!(
                    "[N]={} [P]={} [R]={} [Q]={}: ",
                    ctx.i18n.t("common.next"),
                    ctx.i18n.t("common.previous"),
                    ctx.i18n.t("board.reply"),
                    ctx.i18n.t("common.back")
                ),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "n" => pagination.next(),
                "p" => pagination.prev(),
                "r" => {
                    if session.user_id().is_some() {
                        Self::create_reply(ctx, session, thread_id).await?;
                    } else {
                        ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                            .await?;
                    }
                }
                _ => {}
            }
        }
    }

    /// View a single post.
    async fn run_post_view(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        post_id: i64,
    ) -> Result<ScreenResult> {
        let user_role = Self::get_user_role(ctx, session).await;
        let board_service = BoardService::new(&ctx.db);
        let post = board_service.get_post(post_id, user_role).await?;

        // Check if this board has disable_paging set
        let board = board_service.get_board(post.board_id, user_role).await?;
        let original_paging = ctx.auto_paging_enabled();
        if board.disable_paging {
            ctx.set_auto_paging(false);
        }

        let user_repo = UserRepository::new(ctx.db.pool());
        let author = user_repo
            .get_by_id(post.author_id)
            .await?
            .map(|u| u.nickname)
            .unwrap_or_else(|| "Unknown".to_string());

        let formatted_time = format_datetime(
            &post.created_at,
            &ctx.config.server.timezone,
            "%Y-%m-%d %H:%M",
        );

        let mut context = ctx.create_context();
        context.set("title", Value::string(post.title.as_deref().unwrap_or("(no title)").to_string()));
        context.set("author", Value::string(author));
        context.set("created_at", Value::string(formatted_time));
        context.set("body", Value::string(convert_caret_escape(&post.body)));

        let content = ctx.render_template("board/single_post", &context)?;
        ctx.send(session, &content).await?;

        // Mark this post as read for logged-in users
        if let Some(user_id) = session.user_id() {
            let unread_repo = UnreadRepository::new(ctx.db.pool());
            unread_repo.mark_as_read(user_id, post.board_id, post_id).await?;
        }

        ctx.wait_for_enter(session).await?;

        // Restore auto-paging setting
        ctx.set_auto_paging(original_paging);

        Ok(ScreenResult::Back)
    }

    /// Create a new thread.
    async fn create_thread(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board_id: i64,
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Check rate limit
        match ctx.rate_limiters.post.check(user_id) {
            RateLimitResult::Denied { retry_after } => {
                let msg = ctx.i18n.t_with(
                    "rate_limit.post_denied",
                    &[("seconds", &retry_after.as_secs().to_string())],
                );
                ctx.send_line(session, &msg).await?;
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("board.new_thread")),
        )
        .await?;

        // Get title
        ctx.send(session, &format!("{}: ", ctx.i18n.t("board.title")))
            .await?;
        let title = ctx.read_line(session).await?;
        let title = title.trim();

        if title.is_empty() {
            return Ok(());
        }

        // Create thread using BoardService
        let user_role = Self::get_user_role(ctx, session).await;
        let board_service = BoardService::new(&ctx.db);

        match board_service.create_thread(board_id, title, user_id, user_role).await {
            Ok(_) => {
                // Record successful action for rate limiting
                ctx.rate_limiters.post.record(user_id);
                ctx.send_line(session, ctx.i18n.t("board.thread_created"))
                    .await?;
            }
            Err(e) => {
                error!("Failed to create thread: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Create a reply to a thread.
    async fn create_reply(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        thread_id: i64,
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Check rate limit
        match ctx.rate_limiters.post.check(user_id) {
            RateLimitResult::Denied { retry_after } => {
                let msg = ctx.i18n.t_with(
                    "rate_limit.post_denied",
                    &[("seconds", &retry_after.as_secs().to_string())],
                );
                ctx.send_line(session, &msg).await?;
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("board.reply")))
            .await?;

        // Get body
        ctx.send_line(
            session,
            &format!(
                "{} ({}): ",
                ctx.i18n.t("board.body"),
                ctx.i18n.t("common.end_with_dot")
            ),
        )
        .await?;
        let body = match ctx.read_multiline(session).await? {
            Some(text) => text,
            None => return Ok(()), // Cancelled
        };

        if body.is_empty() {
            return Ok(());
        }

        // Create post using BoardService
        let user_role = Self::get_user_role(ctx, session).await;
        let board_service = BoardService::new(&ctx.db);

        match board_service.create_thread_post(thread_id, user_id, &body, user_role).await {
            Ok(_) => {
                // Record successful action for rate limiting
                ctx.rate_limiters.post.record(user_id);
                ctx.send_line(session, ctx.i18n.t("board.post_created"))
                    .await?;
            }
            Err(e) => {
                error!("Failed to create post: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Create a post in a flat board.
    async fn create_flat_post(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board_id: i64,
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Check rate limit
        match ctx.rate_limiters.post.check(user_id) {
            RateLimitResult::Denied { retry_after } => {
                let msg = ctx.i18n.t_with(
                    "rate_limit.post_denied",
                    &[("seconds", &retry_after.as_secs().to_string())],
                );
                ctx.send_line(session, &msg).await?;
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("board.new_post")),
        )
        .await?;

        // Get title
        ctx.send(session, &format!("{}: ", ctx.i18n.t("board.title")))
            .await?;
        let title = ctx.read_line(session).await?;
        let title = title.trim().to_string();

        // Get body
        ctx.send_line(
            session,
            &format!(
                "{} ({}): ",
                ctx.i18n.t("board.body"),
                ctx.i18n.t("common.end_with_dot")
            ),
        )
        .await?;
        let body = match ctx.read_multiline(session).await? {
            Some(text) => text,
            None => return Ok(()), // Cancelled
        };

        if body.is_empty() {
            return Ok(());
        }

        // Create post using BoardService
        let user_role = Self::get_user_role(ctx, session).await;
        let board_service = BoardService::new(&ctx.db);

        match board_service.create_flat_post(board_id, user_id, &title, &body, user_role).await {
            Ok(_) => {
                // Record successful action for rate limiting
                ctx.rate_limiters.post.record(user_id);
                ctx.send_line(session, ctx.i18n.t("board.post_created"))
                    .await?;
            }
            Err(e) => {
                error!("Failed to create post: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Run unread batch read for a board.
    ///
    /// Displays unread posts one by one, marking each as read after display.
    async fn run_unread_batch_read(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board_id: i64,
    ) -> Result<ScreenResult> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(ScreenResult::Back),
        };

        // Get unread posts (collect into Vec to release the borrow)
        let unread_posts = {
            let unread_repo = UnreadRepository::new(ctx.db.pool());
            unread_repo.get_unread_posts(user_id, board_id).await?
        };

        if unread_posts.is_empty() {
            ctx.send_line(session, "").await?;
            ctx.send_line(session, ctx.i18n.t("board.no_unread"))
                .await?;
            ctx.wait_for_enter(session).await?;
            return Ok(ScreenResult::Back);
        }

        // Check if this board has disable_paging set
        let user_role = Self::get_user_role(ctx, session).await;
        let board_service = BoardService::new(&ctx.db);
        let board = board_service.get_board(board_id, user_role).await?;
        let original_paging = ctx.auto_paging_enabled();
        if board.disable_paging {
            ctx.set_auto_paging(false);
        }

        let total = unread_posts.len();
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &ctx.i18n
                .t_with("board.unread_count", &[("count", &total.to_string())]),
        )
        .await?;
        ctx.send_line(session, "").await?;

        for (index, post) in unread_posts.iter().enumerate() {
            // Display post header (create repo in block to release borrow)
            let author = {
                let user_repo = UserRepository::new(ctx.db.pool());
                user_repo
                    .get_by_id(post.author_id)
                    .await?
                    .map(|u| u.nickname)
                    .unwrap_or_else(|| "Unknown".to_string())
            };

            // Get title: for thread posts, fetch thread title; for flat posts, use post title
            let title = if let Some(thread_id) = post.thread_id {
                let thread_repo = ThreadRepository::new(ctx.db.pool());
                thread_repo
                    .get_by_id(thread_id)
                    .await?
                    .map(|t| t.title)
                    .unwrap_or_else(|| "(no thread)".to_string())
            } else {
                post.title.clone().unwrap_or_else(|| "(no title)".to_string())
            };

            let formatted_time = format_datetime(
                &post.created_at,
                &ctx.config.server.timezone,
                "%Y-%m-%d %H:%M",
            );

            let mut context = ctx.create_context();
            context.set("current", Value::string((index + 1).to_string()));
            context.set("total", Value::string(total.to_string()));
            context.set("title", Value::string(title));
            context.set("author", Value::string(author));
            context.set("created_at", Value::string(formatted_time));
            context.set("body", Value::string(convert_caret_escape(&post.body)));

            let content = ctx.render_template("board/unread_post", &context)?;
            ctx.send(session, &content).await?;

            // Mark this post as read (create repo in block to release borrow)
            {
                let unread_repo = UnreadRepository::new(ctx.db.pool());
                unread_repo.mark_as_read(user_id, board_id, post.id).await?;
            }

            // Restore auto-paging before prompt
            ctx.set_auto_paging(original_paging);

            // Prompt for next action (unless this is the last post)
            if index + 1 < total {
                ctx.send(
                    session,
                    &format!(
                        "[N]={} [Q]={}: ",
                        ctx.i18n.t("common.next"),
                        ctx.i18n.t("common.quit")
                    ),
                )
                .await?;

                let input = ctx.read_line(session).await?;
                let input = input.trim();

                match input.to_ascii_lowercase().as_str() {
                    "q" => {
                        return Ok(ScreenResult::Back);
                    }
                    _ => {
                        // Continue to next post (default for Enter or 'n')
                        ctx.send_line(session, "").await?;
                    }
                }

                // Re-disable paging for next post if needed
                if board.disable_paging {
                    ctx.set_auto_paging(false);
                }
            } else {
                // Last post - show completion message
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("board.unread_complete"))
                    .await?;
                ctx.wait_for_enter(session).await?;
            }
        }

        Ok(ScreenResult::Back)
    }

    /// Mark all posts in a board as read.
    ///
    /// Shows a confirmation prompt before marking all posts as read.
    async fn mark_all_as_read_for_board(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        board_id: i64,
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Show confirmation prompt
        ctx.send_line(session, "").await?;
        ctx.send(session, ctx.i18n.t("board.mark_all_read_confirm"))
            .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim().to_ascii_lowercase();

        if input == "y" || input == "yes" {
            // Mark all posts as read
            let unread_repo = UnreadRepository::new(ctx.db.pool());
            match unread_repo.mark_all_as_read(user_id, board_id).await {
                Ok(true) => {
                    ctx.send_line(session, ctx.i18n.t("board.marked_all_read"))
                        .await?;
                }
                Ok(false) => {
                    // No posts in board
                    ctx.send_line(session, ctx.i18n.t("board.no_posts")).await?;
                }
                Err(e) => {
                    error!("Failed to mark all as read: {}", e);
                    ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Run all unread batch read across all boards.
    ///
    /// Displays unread posts from all boards one by one, marking each as read after display.
    /// Shows the board name for each post.
    async fn run_all_unread_batch_read(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<ScreenResult> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(ScreenResult::Back),
        };

        let user_role = Self::get_user_role(ctx, session).await;

        // Get all unread posts across all boards (collect into Vec to release the borrow)
        let unread_posts: Vec<UnreadPostWithBoard> = {
            let unread_repo = UnreadRepository::new(ctx.db.pool());
            unread_repo.get_all_unread_posts(user_id, user_role).await?
        };

        if unread_posts.is_empty() {
            ctx.send_line(session, "").await?;
            ctx.send_line(session, ctx.i18n.t("board.no_unread_all"))
                .await?;
            ctx.wait_for_enter(session).await?;
            return Ok(ScreenResult::Back);
        }

        let total = unread_posts.len();
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &ctx.i18n
                .t_with("board.unread_all_count", &[("count", &total.to_string())]),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Cache board_id -> disable_paging to avoid repeated DB lookups
        let mut disable_paging_cache: std::collections::HashMap<i64, bool> =
            std::collections::HashMap::new();
        let original_paging = ctx.auto_paging_enabled();

        for (index, unread_post) in unread_posts.iter().enumerate() {
            let post = &unread_post.post;

            // Check disable_paging for this post's board (with cache)
            let disable_paging = if let Some(&cached) = disable_paging_cache.get(&post.board_id) {
                cached
            } else {
                let board_repo = BoardRepository::new(ctx.db.pool());
                let dp = board_repo
                    .get_by_id(post.board_id)
                    .await?
                    .map(|b| b.disable_paging)
                    .unwrap_or(false);
                disable_paging_cache.insert(post.board_id, dp);
                dp
            };

            if disable_paging {
                ctx.set_auto_paging(false);
            } else {
                ctx.set_auto_paging(original_paging);
            }

            // Display post header (create repo in block to release borrow)
            let author = {
                let user_repo = UserRepository::new(ctx.db.pool());
                user_repo
                    .get_by_id(post.author_id)
                    .await?
                    .map(|u| u.nickname)
                    .unwrap_or_else(|| "Unknown".to_string())
            };

            // Get title: for thread posts, fetch thread title; for flat posts, use post title
            let title = if let Some(thread_id) = post.thread_id {
                let thread_repo = ThreadRepository::new(ctx.db.pool());
                thread_repo
                    .get_by_id(thread_id)
                    .await?
                    .map(|t| t.title)
                    .unwrap_or_else(|| "(no thread)".to_string())
            } else {
                post.title.clone().unwrap_or_else(|| "(no title)".to_string())
            };

            let formatted_time = format_datetime(
                &post.created_at,
                &ctx.config.server.timezone,
                "%Y-%m-%d %H:%M",
            );

            let mut context = ctx.create_context();
            context.set("current", Value::string((index + 1).to_string()));
            context.set("total", Value::string(total.to_string()));
            context.set("board_name", Value::string(unread_post.board_name.clone()));
            context.set("title", Value::string(title));
            context.set("author", Value::string(author));
            context.set("created_at", Value::string(formatted_time));
            context.set("body", Value::string(convert_caret_escape(&post.body)));

            let content = ctx.render_template("board/unread_post", &context)?;
            ctx.send(session, &content).await?;

            // Mark this post as read (create repo in block to release borrow)
            {
                let unread_repo = UnreadRepository::new(ctx.db.pool());
                unread_repo.mark_as_read(user_id, post.board_id, post.id).await?;
            }

            // Restore auto-paging before prompt
            ctx.set_auto_paging(original_paging);

            // Prompt for next action (unless this is the last post)
            if index + 1 < total {
                ctx.send(
                    session,
                    &format!(
                        "[N]={} [Q]={}: ",
                        ctx.i18n.t("common.next"),
                        ctx.i18n.t("common.quit")
                    ),
                )
                .await?;

                let input = ctx.read_line(session).await?;
                let input = input.trim();

                match input.to_ascii_lowercase().as_str() {
                    "q" => {
                        return Ok(ScreenResult::Back);
                    }
                    _ => {
                        // Continue to next post (default for Enter or 'n')
                        ctx.send_line(session, "").await?;
                    }
                }
            } else {
                // Last post - show completion message
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("board.unread_all_complete"))
                    .await?;
                ctx.wait_for_enter(session).await?;
            }
        }

        Ok(ScreenResult::Back)
    }

    /// Get user role from session.
    async fn get_user_role(ctx: &ScreenContext, session: &TelnetSession) -> Role {
        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(ctx.db.pool());
            if let Ok(Some(user)) = user_repo.get_by_id(user_id).await {
                return user.role;
            }
        }
        Role::Guest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_screen_exists() {
        // Basic existence test
        let _ = BoardScreen;
    }
}
