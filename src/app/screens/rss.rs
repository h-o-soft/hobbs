//! RSS screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::datetime::format_datetime;
use crate::error::Result;
use crate::rss::{RssFeedRepository, RssItemRepository, RssReadPositionRepository};
use crate::server::TelnetSession;

/// RSS screen handler.
pub struct RssScreen;

impl RssScreen {
    /// Run the RSS feed list screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        let user_id = session.user_id();

        loop {
            // Get feed list with unread counts
            let feeds = RssFeedRepository::list_with_unread(ctx.db.conn(), user_id)?;

            // Calculate total unread
            let total_unread: i64 = feeds.iter().map(|f| f.unread_count).sum();

            // Display feed list
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("rss.title")),
            )
            .await?;

            if total_unread > 0 && user_id.is_some() {
                ctx.send_line(
                    session,
                    &ctx.i18n
                        .t_with("rss.unread_total", &[("count", &total_unread.to_string())]),
                )
                .await?;
            }
            ctx.send_line(session, "").await?;

            if feeds.is_empty() {
                ctx.send_line(session, ctx.i18n.t("rss.no_feeds")).await?;
            } else {
                // Header
                ctx.send_line(
                    session,
                    &format!(
                        "  {:<4} {:<30} {}",
                        ctx.i18n.t("common.number"),
                        ctx.i18n.t("rss.feed_name"),
                        ctx.i18n.t("rss.unread")
                    ),
                )
                .await?;
                ctx.send_line(session, &"-".repeat(50)).await?;

                // Feed list
                for (i, feed_with_unread) in feeds.iter().enumerate() {
                    let num = i + 1;
                    let feed = &feed_with_unread.feed;
                    let unread_count = feed_with_unread.unread_count;

                    let title = if feed.title.chars().count() > 28 {
                        let truncated: String = feed.title.chars().take(25).collect();
                        format!("{}...", truncated)
                    } else {
                        feed.title.clone()
                    };

                    let unread_marker = if unread_count > 0 && user_id.is_some() {
                        format!("{}*", unread_count)
                    } else {
                        "-".to_string()
                    };

                    ctx.send_line(
                        session,
                        &format!("  {:<4} {:<30} {}", num, title, unread_marker),
                    )
                    .await?;
                }
            }

            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!(
                    "{}: {}",
                    ctx.i18n.t("rss.total"),
                    feeds.len()
                ),
            )
            .await?;

            // Prompt
            ctx.send(session, &format!("[Q]={}: ", ctx.i18n.t("common.back")))
                .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        let idx = num as usize - 1;
                        if idx < feeds.len() {
                            Self::show_feed(ctx, session, feeds[idx].feed.id).await?;
                        }
                    }
                }
            }
        }
    }

    /// Show articles in a feed.
    async fn show_feed(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        feed_id: i64,
    ) -> Result<()> {
        let user_id = session.user_id();

        let feed = match RssFeedRepository::get_by_id(ctx.db.conn(), feed_id)? {
            Some(f) => f,
            None => return Ok(()),
        };

        let page_size = 20;
        let mut offset = 0;

        loop {
            // Get items
            let items = RssItemRepository::list_by_feed(ctx.db.conn(), feed_id, page_size, offset)?;
            let total = RssItemRepository::count_by_feed(ctx.db.conn(), feed_id)?;
            let unread_count = user_id
                .map(|uid| RssItemRepository::count_unread(ctx.db.conn(), feed_id, uid))
                .transpose()?
                .unwrap_or(0);

            // Display header
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", feed.title))
                .await?;

            if unread_count > 0 {
                ctx.send_line(
                    session,
                    &ctx.i18n
                        .t_with("rss.unread_count", &[("count", &unread_count.to_string())]),
                )
                .await?;
            }
            ctx.send_line(session, "").await?;

            if items.is_empty() {
                ctx.send_line(session, ctx.i18n.t("rss.no_items")).await?;
            } else {
                // Get read position for this user
                let last_read_id = user_id
                    .and_then(|uid| {
                        RssReadPositionRepository::get(ctx.db.conn(), uid, feed_id)
                            .ok()
                            .flatten()
                    })
                    .and_then(|pos| pos.last_read_item_id);

                // Header
                ctx.send_line(
                    session,
                    &format!(
                        "  {:<4} {:<3} {:<35} {}",
                        ctx.i18n.t("common.number"),
                        "",
                        ctx.i18n.t("rss.article_title"),
                        ctx.i18n.t("rss.date")
                    ),
                )
                .await?;
                ctx.send_line(session, &"-".repeat(60)).await?;

                // Article list
                for (i, item) in items.iter().enumerate() {
                    let num = offset + i + 1;

                    // Check if unread
                    let is_unread = match last_read_id {
                        None => true,
                        Some(last_id) => item.id > last_id,
                    };
                    let unread_marker = if is_unread && user_id.is_some() {
                        "*"
                    } else {
                        " "
                    };

                    let title = if item.title.chars().count() > 33 {
                        let truncated: String = item.title.chars().take(30).collect();
                        format!("{}...", truncated)
                    } else {
                        item.title.clone()
                    };

                    let date = item
                        .published_at
                        .map(|d| {
                            format_datetime(
                                &d.to_rfc3339(),
                                &ctx.config.server.timezone,
                                "%m/%d %H:%M",
                            )
                        })
                        .unwrap_or_else(|| "-".to_string());

                    ctx.send_line(
                        session,
                        &format!("  {:<4} {:<3} {:<35} {}", num, unread_marker, title, date),
                    )
                    .await?;
                }
            }

            ctx.send_line(session, "").await?;

            // Pagination info
            let current_page = offset / page_size + 1;
            let total_pages = (total as usize + page_size - 1) / page_size;
            if total_pages > 1 {
                ctx.send_line(
                    session,
                    &ctx.i18n.t_with(
                        "board.page_of",
                        &[
                            ("current", &current_page.to_string()),
                            ("total", &total_pages.to_string()),
                        ],
                    ),
                )
                .await?;
            }

            // Prompt
            let mut prompt_parts = vec![];
            if offset > 0 {
                prompt_parts.push(format!("[P]={}", ctx.i18n.t("common.previous")));
            }
            if offset + page_size < total as usize {
                prompt_parts.push(format!("[N]={}", ctx.i18n.t("common.next")));
            }
            if unread_count > 0 && user_id.is_some() {
                prompt_parts.push(format!("[U]={}", ctx.i18n.t("rss.read_unread")));
                prompt_parts.push(format!("[A]={}", ctx.i18n.t("rss.mark_all_read")));
            }
            prompt_parts.push(format!("[Q]={}", ctx.i18n.t("common.back")));

            ctx.send(session, &format!("{}: ", prompt_parts.join(" "))).await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(()),
                "p" if offset > 0 => {
                    offset = offset.saturating_sub(page_size);
                }
                "n" if offset + page_size < total as usize => {
                    offset += page_size;
                }
                "u" if unread_count > 0 && user_id.is_some() => {
                    Self::read_unread(ctx, session, feed_id).await?;
                }
                "a" if user_id.is_some() => {
                    Self::mark_all_read(ctx, session, feed_id).await?;
                }
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        let idx = num as usize - 1;
                        if idx >= offset && idx < offset + items.len() {
                            let item_idx = idx - offset;
                            Self::show_item(ctx, session, items[item_idx].id).await?;
                        }
                    }
                }
            }
        }
    }

    /// Show a single article.
    async fn show_item(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        item_id: i64,
    ) -> Result<()> {
        let user_id = session.user_id();

        let item = match RssItemRepository::get_by_id(ctx.db.conn(), item_id)? {
            Some(i) => i,
            None => return Ok(()),
        };

        // Update read position if logged in
        if let Some(uid) = user_id {
            let _ = RssReadPositionRepository::upsert(ctx.db.conn(), uid, item.feed_id, item_id);
        }

        // Display article
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &"=".repeat(60)).await?;
        ctx.send_line(session, &item.title).await?;
        ctx.send_line(session, &"-".repeat(60)).await?;

        if let Some(author) = &item.author {
            ctx.send_line(
                session,
                &format!("{}: {}", ctx.i18n.t("rss.author"), author),
            )
            .await?;
        }

        if let Some(published_at) = item.published_at {
            ctx.send_line(
                session,
                &format!(
                    "{}: {}",
                    ctx.i18n.t("rss.date"),
                    format_datetime(
                        &published_at.to_rfc3339(),
                        &ctx.config.server.timezone,
                        "%Y/%m/%d %H:%M",
                    )
                ),
            )
            .await?;
        }

        if let Some(link) = &item.link {
            ctx.send_line(
                session,
                &ctx.i18n.t_with("rss.view_in_browser", &[("url", link)]),
            )
            .await?;
        }

        ctx.send_line(session, "").await?;

        // Description
        if let Some(description) = &item.description {
            // Word wrap description
            for line in description.lines() {
                if line.is_empty() {
                    ctx.send_line(session, "").await?;
                } else {
                    // Simple word wrap at 70 chars
                    let mut current = String::new();
                    for word in line.split_whitespace() {
                        if current.len() + word.len() + 1 > 70 {
                            ctx.send_line(session, &current).await?;
                            current = word.to_string();
                        } else {
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push_str(word);
                        }
                    }
                    if !current.is_empty() {
                        ctx.send_line(session, &current).await?;
                    }
                }
            }
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &"=".repeat(60)).await?;

        // Wait for user
        ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
        ctx.read_line(session).await?;

        Ok(())
    }

    /// Read unread articles one by one.
    async fn read_unread(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        feed_id: i64,
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        loop {
            // Get current unread count
            let unread_count = RssItemRepository::count_unread(ctx.db.conn(), feed_id, user_id)?;
            if unread_count == 0 {
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("rss.reading_complete"))
                    .await?;
                ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
                ctx.read_line(session).await?;
                break;
            }

            // Get the oldest unread item
            let last_read_id = RssReadPositionRepository::get(ctx.db.conn(), user_id, feed_id)?
                .and_then(|pos| pos.last_read_item_id);

            // Get items after last read
            let items = RssItemRepository::list_by_feed(ctx.db.conn(), feed_id, 100, 0)?;
            let unread_item = items.into_iter().rev().find(|item| {
                match last_read_id {
                    None => true,
                    Some(last_id) => item.id > last_id,
                }
            });

            let item = match unread_item {
                Some(i) => i,
                None => break,
            };

            // Display article
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &ctx.i18n.t_with("rss.unread_count", &[("count", &unread_count.to_string())]),
            )
            .await?;
            ctx.send_line(session, &"=".repeat(60)).await?;
            ctx.send_line(session, &item.title).await?;
            ctx.send_line(session, &"-".repeat(60)).await?;

            if let Some(author) = &item.author {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("rss.author"), author),
                )
                .await?;
            }

            if let Some(published_at) = item.published_at {
                ctx.send_line(
                    session,
                    &format!(
                        "{}: {}",
                        ctx.i18n.t("rss.date"),
                        format_datetime(
                            &published_at.to_rfc3339(),
                            &ctx.config.server.timezone,
                            "%Y/%m/%d %H:%M",
                        )
                    ),
                )
                .await?;
            }

            if let Some(link) = &item.link {
                ctx.send_line(
                    session,
                    &ctx.i18n.t_with("rss.view_in_browser", &[("url", link)]),
                )
                .await?;
            }

            ctx.send_line(session, "").await?;

            if let Some(description) = &item.description {
                for line in description.lines() {
                    if line.is_empty() {
                        ctx.send_line(session, "").await?;
                    } else {
                        let mut current = String::new();
                        for word in line.split_whitespace() {
                            if current.len() + word.len() + 1 > 70 {
                                ctx.send_line(session, &current).await?;
                                current = word.to_string();
                            } else {
                                if !current.is_empty() {
                                    current.push(' ');
                                }
                                current.push_str(word);
                            }
                        }
                        if !current.is_empty() {
                            ctx.send_line(session, &current).await?;
                        }
                    }
                }
            }

            ctx.send_line(session, "").await?;
            ctx.send_line(session, &"=".repeat(60)).await?;

            // Update read position
            let _ = RssReadPositionRepository::upsert(ctx.db.conn(), user_id, feed_id, item.id);

            // Prompt for next
            ctx.send(session, ctx.i18n.t("rss.press_enter_next")).await?;
            let input = ctx.read_line(session).await?;
            if input.trim().eq_ignore_ascii_case("q") {
                break;
            }
        }

        Ok(())
    }

    /// Mark all articles as read.
    async fn mark_all_read(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        feed_id: i64,
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Confirm
        ctx.send(session, ctx.i18n.t("rss.mark_all_read_confirm"))
            .await?;
        let input = ctx.read_line(session).await?;

        if input.trim().eq_ignore_ascii_case("y") {
            RssReadPositionRepository::mark_all_as_read(ctx.db.conn(), user_id, feed_id)?;
            ctx.send_line(session, ctx.i18n.t("rss.marked_all_read"))
                .await?;
        }

        Ok(())
    }
}
