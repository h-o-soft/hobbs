//! Profile screen handler.

use tracing::error;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::auth::{change_password, update_profile, ProfileUpdateRequest};
use crate::db::UserRepository;
use crate::error::Result;
use crate::server::{EchoMode, TelnetSession};

/// Profile screen handler.
pub struct ProfileScreen;

impl ProfileScreen {
    /// Run the profile screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        let user_id = match session.user_id() {
            Some(id) => id,
            None => {
                ctx.send_line(session, ctx.i18n.t("menu.login_required"))
                    .await?;
                return Ok(ScreenResult::Back);
            }
        };

        loop {
            // Get user info
            let user_repo = UserRepository::new(&ctx.db);
            let user = match user_repo.get_by_id(user_id)? {
                Some(u) => u,
                None => return Ok(ScreenResult::Back),
            };

            // Display profile
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("profile.title")))
                .await?;
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("{}: {}", ctx.i18n.t("profile.username"), user.username),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("{}: {}", ctx.i18n.t("profile.nickname"), user.nickname),
            )
            .await?;
            ctx.send_line(
                session,
                &format!(
                    "{}: {}",
                    ctx.i18n.t("auth.email"),
                    user.email.as_deref().unwrap_or("-")
                ),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("{}: {:?}", ctx.i18n.t("role.member"), user.role),
            )
            .await?;
            ctx.send_line(
                session,
                &format!(
                    "{}: {}",
                    ctx.i18n.t("profile.member_since"),
                    user.created_at
                ),
            )
            .await?;
            ctx.send_line(
                session,
                &format!(
                    "{}: {}",
                    ctx.i18n.t("profile.last_login"),
                    user.last_login.as_deref().unwrap_or("-")
                ),
            )
            .await?;

            if let Some(ref profile_text) = user.profile {
                ctx.send_line(session, "").await?;
                ctx.send_line(session, &format!("--- {} ---", ctx.i18n.t("profile.bio")))
                    .await?;
                ctx.send_line(session, profile_text).await?;
            }

            ctx.send_line(session, "").await?;

            // Options
            ctx.send(
                session,
                &format!(
                    "[E]={} [P]={} [Q]={}: ",
                    ctx.i18n.t("profile.edit"),
                    ctx.i18n.t("profile.change_password"),
                    ctx.i18n.t("common.back")
                ),
            )
            .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(ScreenResult::Back),
                "e" => {
                    Self::edit_profile(ctx, session, user_id).await?;
                }
                "p" => {
                    Self::change_password(ctx, session, user_id).await?;
                }
                _ => {}
            }
        }
    }

    /// Edit profile.
    async fn edit_profile(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        user_id: i64,
    ) -> Result<()> {
        // Get user info first
        let (current_nickname, current_email) = {
            let user_repo = UserRepository::new(&ctx.db);
            let user = match user_repo.get_by_id(user_id)? {
                Some(u) => u,
                None => return Ok(()),
            };
            (user.nickname.clone(), user.email.clone())
        };

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("profile.edit")))
            .await?;
        ctx.send_line(session, ctx.i18n.t("common.press_enter"))
            .await?;
        ctx.send_line(session, "").await?;

        // Edit nickname
        ctx.send(
            session,
            &format!(
                "{} [{}]: ",
                ctx.i18n.t("profile.nickname"),
                current_nickname
            ),
        )
        .await?;
        let nickname = ctx.read_line(session).await?;
        let nickname = nickname.trim();
        let new_nickname = if nickname.is_empty() {
            None
        } else {
            Some(nickname.to_string())
        };

        // Edit email
        ctx.send(
            session,
            &format!(
                "{} [{}]: ",
                ctx.i18n.t("auth.email"),
                current_email.as_deref().unwrap_or("-")
            ),
        )
        .await?;
        let email = ctx.read_line(session).await?;
        let email = email.trim();
        let new_email = if email.is_empty() {
            None
        } else if email == "-" {
            Some(None) // Clear email
        } else {
            Some(Some(email.to_string()))
        };

        // Edit profile text
        ctx.send_line(
            session,
            &format!(
                "{} ({}): ",
                ctx.i18n.t("profile.bio"),
                ctx.i18n.t("common.end_with_dot")
            ),
        )
        .await?;
        let profile_text = Self::read_multiline(ctx, session).await?;
        let new_profile = if profile_text.is_empty() {
            None
        } else {
            Some(Some(profile_text))
        };

        // Build update request
        let mut request = ProfileUpdateRequest::new();
        if let Some(nick) = new_nickname {
            request = request.nickname(nick);
        }
        if let Some(email_opt) = new_email {
            request = request.email(email_opt);
        }
        if let Some(profile_opt) = new_profile {
            request = request.profile(profile_opt);
        }

        // Apply update - create a new user_repo for this operation
        let user_repo = UserRepository::new(&ctx.db);
        match update_profile(&user_repo, user_id, request) {
            Ok(_) => {
                ctx.send_line(session, ctx.i18n.t("profile.profile_updated"))
                    .await?;
            }
            Err(e) => {
                error!("Failed to update profile: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Change password.
    async fn change_password(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        user_id: i64,
    ) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("profile.change_password")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Get current password
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("auth.current_password")),
        )
        .await?;
        ctx.set_echo_mode(EchoMode::Password);
        let current = ctx.read_line(session).await?;
        ctx.set_echo_mode(EchoMode::Normal);
        ctx.send_line(session, "").await?;

        // Get new password
        ctx.send(session, &format!("{}: ", ctx.i18n.t("auth.new_password")))
            .await?;
        ctx.set_echo_mode(EchoMode::Password);
        let new_password = ctx.read_line(session).await?;
        ctx.set_echo_mode(EchoMode::Normal);
        ctx.send_line(session, "").await?;

        // Confirm new password
        ctx.send(
            session,
            &format!("{}: ", ctx.i18n.t("auth.password_confirm")),
        )
        .await?;
        ctx.set_echo_mode(EchoMode::Password);
        let confirm = ctx.read_line(session).await?;
        ctx.set_echo_mode(EchoMode::Normal);
        ctx.send_line(session, "").await?;

        // Validate
        if new_password != confirm {
            ctx.send_line(session, ctx.i18n.t("auth.password_mismatch"))
                .await?;
            return Ok(());
        }

        // Change password
        let user_repo = UserRepository::new(&ctx.db);
        match change_password(&user_repo, user_id, &current, &new_password) {
            Ok(()) => {
                ctx.send_line(session, ctx.i18n.t("auth.password_changed"))
                    .await?;
            }
            Err(e) => {
                error!("Failed to change password: {}", e);
                ctx.send_line(session, ctx.i18n.t("auth.password_incorrect"))
                    .await?;
            }
        }

        Ok(())
    }

    /// Read multiline input.
    async fn read_multiline(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
    ) -> Result<String> {
        let mut lines = Vec::new();

        loop {
            ctx.send(session, "> ").await?;
            let line = ctx.read_line(session).await?;

            if line.trim() == "." {
                break;
            }

            lines.push(line);
        }

        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_screen_exists() {
        let _ = ProfileScreen;
    }
}
