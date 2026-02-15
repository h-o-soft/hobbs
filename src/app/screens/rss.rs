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
use crate::template::Value;

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

            // Display feed list using template
            let mut context = ctx.create_context();
            let show_unread = total_unread > 0 && user_id.is_some();
            context.set("show_unread_total", Value::bool(show_unread));
            if show_unread {
                context.set("unread_total_text", Value::string(
                    ctx.i18n.t_with("rss.unread_total", &[("count", &total_unread.to_string())]),
                ));
            }
            context.set("has_feeds", Value::bool(!feeds.is_empty()));

            if !feeds.is_empty() {
                let mut feed_list = Vec::new();
                for (i, feed_with_unread) in feeds.iter().enumerate() {
                    let num = i + 1;
                    let feed = &feed_with_unread.feed;
                    let unread_count = feed_with_unread.unread_count;

                    let unread_marker = if unread_count > 0 && user_id.is_some() {
                        format!("{}*", unread_count)
                    } else {
                        "-".to_string()
                    };

                    let mut entry = std::collections::HashMap::new();
                    entry.insert("number".to_string(), Value::string(num.to_string()));
                    entry.insert("title".to_string(), Value::string(&feed.title));
                    entry.insert("unread_marker".to_string(), Value::string(unread_marker));
                    feed_list.push(Value::Object(entry));
                }
                context.set("feeds", Value::List(feed_list));
            }
            context.set("total_text", Value::string(
                format!("{}: {}", ctx.i18n.t("rss.total"), feeds.len()),
            ));

            let content = ctx.render_template("rss/list", &context)?;
            ctx.send(session, &content).await?;

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

            // Display article list using template
            let mut context = ctx.create_context();
            context.set("feed_title", Value::string(&feed.title));
            context.set("has_unread", Value::bool(unread_count > 0));
            if unread_count > 0 {
                context.set("unread_count_text", Value::string(
                    ctx.i18n.t_with("rss.unread_count", &[("count", &unread_count.to_string())]),
                ));
            }
            context.set("has_items", Value::bool(!items.is_empty()));

            if !items.is_empty() {
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

                let mut item_list = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let num = offset + i + 1;

                    let is_unread = match last_read_id {
                        None => true,
                        Some(last_id) => item.id > last_id,
                    };
                    let unread_marker = if is_unread && user_id.is_some() {
                        "*"
                    } else {
                        " "
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

                    let mut entry = std::collections::HashMap::new();
                    entry.insert("number".to_string(), Value::string(num.to_string()));
                    entry.insert("unread_marker".to_string(), Value::string(unread_marker));
                    entry.insert("title".to_string(), Value::string(&item.title));
                    entry.insert("date".to_string(), Value::string(date));
                    item_list.push(Value::Object(entry));
                }
                context.set("items", Value::List(item_list));
            }

            // Pagination info
            let current_page = offset / page_size + 1;
            let total_pages = (total as usize + page_size - 1) / page_size;
            if total_pages > 1 {
                context.set("page_info", Value::string(
                    ctx.i18n.t_with(
                        "board.page_of",
                        &[
                            ("current", &current_page.to_string()),
                            ("total", &total_pages.to_string()),
                        ],
                    ),
                ));
            }

            let content = ctx.render_template("rss/feed", &context)?;
            ctx.send(session, &content).await?;

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

        // Display article using template
        let mut context = ctx.create_context();
        context.set("title", Value::string(&item.title));
        context.set("author_label", Value::string(ctx.i18n.t("rss.author")));
        context.set("date_label", Value::string(ctx.i18n.t("rss.date")));

        if let Some(author) = &item.author {
            context.set("author", Value::string(author));
        }

        if let Some(published_at) = item.published_at {
            context.set("date", Value::string(format_datetime(
                &published_at.to_rfc3339(),
                &ctx.config.server.timezone,
                "%Y/%m/%d %H:%M",
            )));
        }

        if let Some(link) = &item.link {
            context.set("link_text", Value::string(
                ctx.i18n.t_with("rss.view_in_browser", &[("url", link)]),
            ));
        }

        let desc_text = item.description.as_deref().unwrap_or_default();
        context.set("description", Value::string(desc_text));

        let content = ctx.render_template("rss/item", &context)?;
        ctx.send(session, &content).await?;

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

            // Get the oldest unread item (by id ASC)
            let read_pos_repo = RssReadPositionRepository::new(ctx.db.pool());
            let item = match item_repo.get_next_unread(feed_id, user_id).await? {
                Some(i) => i,
                None => break,
            };

            // Display article using template
            let mut context = ctx.create_context();
            context.set("unread_info", Value::string(
                ctx.i18n.t_with("rss.unread_count", &[("count", &unread_count.to_string())]),
            ));
            context.set("title", Value::string(&item.title));
            context.set("author_label", Value::string(ctx.i18n.t("rss.author")));
            context.set("date_label", Value::string(ctx.i18n.t("rss.date")));

            if let Some(author) = &item.author {
                context.set("author", Value::string(author));
            }

            if let Some(published_at) = item.published_at {
                context.set("date", Value::string(format_datetime(
                    &published_at.to_rfc3339(),
                    &ctx.config.server.timezone,
                    "%Y/%m/%d %H:%M",
                )));
            }

            if let Some(link) = &item.link {
                context.set("link_text", Value::string(
                    ctx.i18n.t_with("rss.view_in_browser", &[("url", link)]),
                ));
            }

            // Word wrap description to terminal width
            let desc_text = item
                .description
                .as_deref()
                .map(|d| ctx.word_wrap(d))
                .unwrap_or_default();
            context.set("description", Value::string(&desc_text));

            let content = ctx.render_template("rss/item", &context)?;
            ctx.send(session, &content).await?;

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
