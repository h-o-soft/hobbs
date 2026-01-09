//! RSS screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::datetime::format_datetime;
use crate::error::Result;
use crate::rss::{
    fetch_feed, validate_url, NewRssFeed, NewRssItem, RssFeedRepository, RssItemRepository,
    RssReadPositionRepository, MAX_ITEMS_PER_FEED,
};
use crate::server::TelnetSession;

/// RSS screen handler.
pub struct RssScreen;

impl RssScreen {
    /// Run the RSS feed list screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        let user_id = session.user_id();

        loop {
            // Get feed list with unread counts
            let feed_repo = RssFeedRepository::new(ctx.db.pool());
            let feeds = feed_repo.list_with_unread(user_id).await?;

            // Calculate total unread
            let total_unread: i64 = feeds.iter().map(|f| f.unread_count).sum();

            // Display feed list
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("rss.title")))
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
                &format!("{}: {}", ctx.i18n.t("rss.total"), feeds.len()),
            )
            .await?;

            // Prompt - show add/delete options for logged-in users
            let prompt = if user_id.is_some() {
                format!(
                    "[A]={} [D]={} [Q]={}: ",
                    ctx.i18n.t("rss.add_feed"),
                    ctx.i18n.t("rss.delete_feed"),
                    ctx.i18n.t("common.back")
                )
            } else {
                format!("[Q]={}: ", ctx.i18n.t("common.back"))
            };
            ctx.send(session, &prompt).await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "a" if user_id.is_some() => {
                    Self::add_feed(ctx, session).await?;
                }
                "d" if user_id.is_some() => {
                    Self::delete_feed(ctx, session, &feeds).await?;
                }
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        let idx = num as usize - 1;
                        if idx < feeds.len() {
                            // Check ownership before showing feed
                            let feed = &feeds[idx].feed;
                            if Some(feed.created_by) == user_id || user_id.is_none() {
                                Self::show_feed(ctx, session, feed.id).await?;
                            }
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

        let feed_repo = RssFeedRepository::new(ctx.db.pool());
        let feed = match feed_repo.get_by_id(feed_id).await? {
            Some(f) => f,
            None => return Ok(()),
        };

        let page_size = 20;
        let mut offset = 0;

        loop {
            // Get items
            let item_repo = RssItemRepository::new(ctx.db.pool());
            let items = item_repo.list_by_feed(feed_id, page_size, offset).await?;
            let total = item_repo.count_by_feed(feed_id).await?;
            let unread_count = match user_id {
                Some(uid) => item_repo.count_unread(feed_id, uid).await?,
                None => 0,
            };

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
                let read_pos_repo = RssReadPositionRepository::new(ctx.db.pool());
                let last_read_id = match user_id {
                    Some(uid) => read_pos_repo
                        .get(uid, feed_id)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|pos| pos.last_read_item_id),
                    None => None,
                };

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

            ctx.send(session, &format!("{}: ", prompt_parts.join(" ")))
                .await?;

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

        let item_repo = RssItemRepository::new(ctx.db.pool());
        let item = match item_repo.get_by_id(item_id).await? {
            Some(i) => i,
            None => return Ok(()),
        };

        // Update read position if logged in
        if let Some(uid) = user_id {
            let read_pos_repo = RssReadPositionRepository::new(ctx.db.pool());
            let _ = read_pos_repo.upsert(uid, item.feed_id, item_id).await;
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
            let item_repo = RssItemRepository::new(ctx.db.pool());
            let unread_count = item_repo.count_unread(feed_id, user_id).await?;
            if unread_count == 0 {
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("rss.reading_complete"))
                    .await?;
                ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
                ctx.read_line(session).await?;
                break;
            }

            // Get the oldest unread item
            let read_pos_repo = RssReadPositionRepository::new(ctx.db.pool());
            let last_read_id = read_pos_repo
                .get(user_id, feed_id)
                .await?
                .and_then(|pos| pos.last_read_item_id);

            // Get items after last read
            let items = item_repo.list_by_feed(feed_id, 100, 0).await?;
            let unread_item = items.into_iter().rev().find(|item| match last_read_id {
                None => true,
                Some(last_id) => item.id > last_id,
            });

            let item = match unread_item {
                Some(i) => i,
                None => break,
            };

            // Display article
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &ctx.i18n
                    .t_with("rss.unread_count", &[("count", &unread_count.to_string())]),
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
            let _ = read_pos_repo.upsert(user_id, feed_id, item.id).await;

            // Prompt for next
            ctx.send(session, ctx.i18n.t("rss.press_enter_next"))
                .await?;
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
            let read_pos_repo = RssReadPositionRepository::new(ctx.db.pool());
            read_pos_repo.mark_all_as_read(user_id, feed_id).await?;
            ctx.send_line(session, ctx.i18n.t("rss.marked_all_read"))
                .await?;
        }

        Ok(())
    }

    /// Add a new RSS feed.
    async fn add_feed(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("rss.add_feed")))
            .await?;
        ctx.send_line(session, "").await?;

        // Get URL
        ctx.send(session, &format!("{}: ", ctx.i18n.t("rss.enter_url")))
            .await?;
        let url = ctx.read_line(session).await?;
        let url = url.trim();

        if url.is_empty() {
            return Ok(());
        }

        // Validate URL
        if let Err(e) = validate_url(url) {
            ctx.send_line(session, &format!("{}: {}", ctx.i18n.t("common.error"), e))
                .await?;
            ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
            ctx.read_line(session).await?;
            return Ok(());
        }

        // Check if already subscribed
        let feed_repo = RssFeedRepository::new(ctx.db.pool());
        if feed_repo.get_by_user_url(user_id, url).await?.is_some() {
            ctx.send_line(session, ctx.i18n.t("rss.already_subscribed"))
                .await?;
            ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
            ctx.read_line(session).await?;
            return Ok(());
        }

        // Fetch and parse feed
        ctx.send_line(session, ctx.i18n.t("rss.fetching")).await?;

        match fetch_feed(url).await {
            Ok(parsed) => {
                // Create feed record
                let mut new_feed = NewRssFeed::new(url, &parsed.title, user_id);
                if let Some(desc) = parsed.description {
                    new_feed = new_feed.with_description(desc);
                }
                if let Some(site_url) = parsed.site_url {
                    new_feed = new_feed.with_site_url(site_url);
                }

                match feed_repo.create(&new_feed).await {
                    Ok(feed) => {
                        // Store initial items
                        let item_repo = RssItemRepository::new(ctx.db.pool());
                        for item in parsed.items.into_iter().take(MAX_ITEMS_PER_FEED) {
                            let mut new_item = NewRssItem::new(feed.id, &item.guid, &item.title);
                            if let Some(link) = item.link {
                                new_item = new_item.with_link(link);
                            }
                            if let Some(desc) = item.description {
                                new_item = new_item.with_description(desc);
                            }
                            if let Some(author) = item.author {
                                new_item = new_item.with_author(author);
                            }
                            if let Some(pub_date) = item.published_at {
                                new_item = new_item.with_published_at(pub_date);
                            }
                            let _ = item_repo.create_or_ignore(&new_item).await;
                        }

                        ctx.send_line(
                            session,
                            &ctx.i18n.t_with("rss.feed_added", &[("title", &feed.title)]),
                        )
                        .await?;
                    }
                    Err(e) => {
                        ctx.send_line(session, &format!("{}: {}", ctx.i18n.t("common.error"), e))
                            .await?;
                    }
                }
            }
            Err(e) => {
                ctx.send_line(
                    session,
                    &ctx.i18n
                        .t_with("rss.fetch_error", &[("error", &e.to_string())]),
                )
                .await?;
            }
        }

        ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
        ctx.read_line(session).await?;

        Ok(())
    }

    /// Delete an RSS feed.
    async fn delete_feed(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        feeds: &[crate::rss::RssFeedWithUnread],
    ) -> Result<()> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        if feeds.is_empty() {
            ctx.send_line(session, ctx.i18n.t("rss.no_feeds")).await?;
            ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
            ctx.read_line(session).await?;
            return Ok(());
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("rss.delete_feed")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Show numbered list
        for (i, feed_with_unread) in feeds.iter().enumerate() {
            let feed = &feed_with_unread.feed;
            ctx.send_line(session, &format!("  {}: {}", i + 1, feed.title))
                .await?;
        }
        ctx.send_line(session, "").await?;

        // Get selection
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("rss.enter_feed_number")),
        )
        .await?;
        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.is_empty() {
            return Ok(());
        }

        if let Some(num) = ctx.parse_number(input) {
            let idx = num as usize - 1;
            if idx < feeds.len() {
                let feed = &feeds[idx].feed;

                // Check ownership
                if feed.created_by != user_id {
                    ctx.send_line(session, ctx.i18n.t("rss.not_your_feed"))
                        .await?;
                    ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
                    ctx.read_line(session).await?;
                    return Ok(());
                }

                // Confirm
                ctx.send(
                    session,
                    &ctx.i18n
                        .t_with("rss.confirm_delete", &[("title", &feed.title)]),
                )
                .await?;
                let confirm = ctx.read_line(session).await?;

                if confirm.trim().eq_ignore_ascii_case("y") {
                    let feed_repo = RssFeedRepository::new(ctx.db.pool());
                    match feed_repo.delete(feed.id).await {
                        Ok(_) => {
                            ctx.send_line(
                                session,
                                &ctx.i18n
                                    .t_with("rss.feed_deleted", &[("title", &feed.title)]),
                            )
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
                    ctx.send(session, ctx.i18n.t("common.press_enter")).await?;
                    ctx.read_line(session).await?;
                }
            }
        }

        Ok(())
    }
}
