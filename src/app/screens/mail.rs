//! Mail screen handler.

use tracing::error;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::datetime::format_utc_datetime;
use crate::db::UserRepository;
use crate::error::Result;
use crate::mail::{MailRepository, NewMail};
use crate::rate_limit::RateLimitResult;
use crate::server::{convert_caret_escape, TelnetSession};

/// Mail screen handler.
pub struct MailScreen;

impl MailScreen {
    /// Run the mail inbox screen.
    pub async fn run_inbox(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<ScreenResult> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => {
                ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                    .await?;
                return Ok(ScreenResult::Back);
            }
        };

        loop {
            // Get mail list (no pagination in repository)
            let mail_repo = MailRepository::new(ctx.db.pool());
            let mails = mail_repo.list_inbox(user_id).await?;
            let total = mails.len();

            // Display mail list
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("mail.inbox")))
                .await?;
            ctx.send_line(session, "").await?;

            if mails.is_empty() {
                ctx.send_line(session, ctx.i18n.t("mail.no_mail")).await?;
            } else {
                let user_repo = UserRepository::new(ctx.db.pool());
                ctx.send_line(
                    session,
                    &format!(
                        "  {:<4} {:<3} {:<12} {:<20}",
                        ctx.i18n.t("common.number"),
                        "",
                        ctx.i18n.t("mail.from"),
                        ctx.i18n.t("mail.subject")
                    ),
                )
                .await?;
                ctx.send_line(session, &"-".repeat(50)).await?;

                for (i, mail) in mails.iter().enumerate() {
                    let num = i + 1;
                    let unread = if mail.is_read { " " } else { "*" };
                    let from = user_repo
                        .get_by_id(mail.sender_id)
                        .await?
                        .map(|u| u.nickname)
                        .unwrap_or_else(|| "Unknown".to_string());
                    let from = if from.chars().count() > 10 {
                        let truncated: String = from.chars().take(8).collect();
                        format!("{}...", truncated)
                    } else {
                        from
                    };
                    let subject = if mail.subject.chars().count() > 18 {
                        let truncated: String = mail.subject.chars().take(15).collect();
                        format!("{}...", truncated)
                    } else {
                        mail.subject.clone()
                    };

                    ctx.send_line(
                        session,
                        &format!("  {:<4} {:<3} {:<12} {:<20}", num, unread, from, subject),
                    )
                    .await?;
                }
            }

            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("{}: {}", ctx.i18n.t("mail.total"), total))
                .await?;

            // Prompt
            ctx.send(
                session,
                &format!(
                    "[W]={} [Q]={}: ",
                    ctx.i18n.t("mail.compose"),
                    ctx.i18n.t("common.back")
                ),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "w" => {
                    Self::compose(ctx, session, user_id).await?;
                }
                _ => {
                    if let Some(num) = ctx.parse_number(input) {
                        let idx = num as usize - 1;
                        if idx < mails.len() {
                            Self::view_mail(ctx, session, mails[idx].id, user_id).await?;
                        }
                    }
                }
            }
        }
    }

    /// View a mail.
    async fn view_mail(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        mail_id: i64,
        user_id: i64,
    ) -> Result<()> {
        // Get mail and prepare data in a separate scope
        let (mail, from_name, to_name) = {
            let mail_repo = MailRepository::new(ctx.db.pool());
            let mail = match mail_repo.get_by_id(mail_id).await? {
                Some(m) => m,
                None => return Ok(()),
            };

            // Check ownership
            if mail.recipient_id != user_id && mail.sender_id != user_id {
                return Ok(());
            }

            // Mark as read
            if !mail.is_read && mail.recipient_id == user_id {
                mail_repo.mark_as_read(mail_id).await?;
            }

            // Get sender/recipient names
            let user_repo = UserRepository::new(ctx.db.pool());
            let from_name = user_repo
                .get_by_id(mail.sender_id)
                .await?
                .map(|u| u.nickname)
                .unwrap_or_else(|| "Unknown".to_string());
            let to_name = user_repo
                .get_by_id(mail.recipient_id)
                .await?
                .map(|u| u.nickname)
                .unwrap_or_else(|| "Unknown".to_string());

            (mail, from_name, to_name)
        };

        // Display mail
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("mail.inbox")))
            .await?;
        ctx.send_line(
            session,
            &format!("{}: {}", ctx.i18n.t("mail.from"), from_name),
        )
        .await?;
        ctx.send_line(session, &format!("{}: {}", ctx.i18n.t("mail.to"), to_name))
            .await?;
        ctx.send_line(
            session,
            &format!("{}: {}", ctx.i18n.t("mail.subject"), mail.subject),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("mail.date"),
                format_utc_datetime(&mail.created_at, &ctx.config.server.timezone, "%Y/%m/%d %H:%M")
            ),
        )
        .await?;
        ctx.send_line(session, &"-".repeat(40)).await?;
        ctx.send_line(session, &convert_caret_escape(&mail.body))
            .await?;
        ctx.send_line(session, &"-".repeat(40)).await?;

        // Options
        loop {
            ctx.send(
                session,
                &format!(
                    "[R]={} [D]={} [Q]={}: ",
                    ctx.i18n.t("mail.reply"),
                    ctx.i18n.t("mail.delete"),
                    ctx.i18n.t("common.back")
                ),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(()),
                "r" => {
                    Self::reply(ctx, session, &mail, user_id).await?;
                    return Ok(());
                }
                "d" => {
                    let mail_repo = MailRepository::new(ctx.db.pool());
                    mail_repo.delete_by_user(mail_id, user_id).await?;
                    ctx.send_line(session, ctx.i18n.t("mail.mail_deleted"))
                        .await?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    /// Compose a new mail.
    async fn compose(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        from_id: i64,
    ) -> Result<()> {
        // Check rate limit
        match ctx.rate_limiters.mail.check(from_id) {
            RateLimitResult::Denied { retry_after } => {
                let msg = ctx.i18n.t_with(
                    "rate_limit.mail_denied",
                    &[("seconds", &retry_after.as_secs().to_string())],
                );
                ctx.send_line(session, &msg).await?;
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("mail.compose")))
            .await?;

        // Get recipient
        ctx.send(session, &format!("{}: ", ctx.i18n.t("mail.to")))
            .await?;
        let to_name = ctx.read_line(session).await?;
        let to_name = to_name.trim();

        if to_name.is_empty() {
            return Ok(());
        }

        // Find recipient
        let user_repo = UserRepository::new(ctx.db.pool());
        let to_user = match user_repo.get_by_username(to_name).await? {
            Some(u) => u,
            None => {
                ctx.send_line(session, ctx.i18n.t("mail.recipient_not_found"))
                    .await?;
                return Ok(());
            }
        };

        // Get subject
        ctx.send(session, &format!("{}: ", ctx.i18n.t("mail.subject")))
            .await?;
        let subject = ctx.read_line(session).await?;
        let subject = subject.trim();

        if subject.is_empty() {
            return Ok(());
        }

        // Get body
        ctx.send_line(
            session,
            &format!(
                "{} ({}): ",
                ctx.i18n.t("mail.body"),
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

        // Send mail
        let new_mail = NewMail::new(from_id, to_user.id, subject, &body);
        let mail_repo = MailRepository::new(ctx.db.pool());

        match mail_repo.create(&new_mail).await {
            Ok(_) => {
                // Record successful action for rate limiting
                ctx.rate_limiters.mail.record(from_id);
                ctx.send_line(session, ctx.i18n.t("mail.mail_sent")).await?;
            }
            Err(e) => {
                error!("Failed to send mail: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Reply to a mail.
    async fn reply(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        original: &crate::mail::Mail,
        from_id: i64,
    ) -> Result<()> {
        // Check rate limit
        match ctx.rate_limiters.mail.check(from_id) {
            RateLimitResult::Denied { retry_after } => {
                let msg = ctx.i18n.t_with(
                    "rate_limit.mail_denied",
                    &[("seconds", &retry_after.as_secs().to_string())],
                );
                ctx.send_line(session, &msg).await?;
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("mail.reply")))
            .await?;

        // Get subject
        let default_subject = format!("Re: {}", original.subject);
        ctx.send(
            session,
            &format!("{} [{}]: ", ctx.i18n.t("mail.subject"), default_subject),
        )
        .await?;
        let subject = ctx.read_line(session).await?;
        let subject = if subject.trim().is_empty() {
            default_subject
        } else {
            subject.trim().to_string()
        };

        // Get body
        ctx.send_line(
            session,
            &format!(
                "{} ({}): ",
                ctx.i18n.t("mail.body"),
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

        // Send reply (reply to sender)
        let new_mail = NewMail::new(from_id, original.sender_id, &subject, &body);
        let mail_repo = MailRepository::new(ctx.db.pool());

        match mail_repo.create(&new_mail).await {
            Ok(_) => {
                // Record successful action for rate limiting
                ctx.rate_limiters.mail.record(from_id);
                ctx.send_line(session, ctx.i18n.t("mail.mail_sent")).await?;
            }
            Err(e) => {
                error!("Failed to send reply: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_screen_exists() {
        let _ = MailScreen;
    }
}
