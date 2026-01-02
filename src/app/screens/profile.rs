//! Profile screen handler.

use tracing::error;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::auth::{change_password, update_profile, ProfileUpdateRequest};
use crate::db::{Role, UserRepository, UserUpdate};
use crate::error::Result;
use crate::server::{CharacterEncoding, EchoMode, TelnetSession};
use crate::template::Value;

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

            // Display profile using template
            let mut context = ctx.create_context();
            context.set("user.username", Value::string(user.username.clone()));
            context.set("user.nickname", Value::string(user.nickname.clone()));
            context.set(
                "user.email",
                Value::string(user.email.as_deref().unwrap_or("-").to_string()),
            );
            context.set("user.role_name", Value::string(Self::role_name(ctx, user.role)));
            context.set("user.created_at", Value::string(user.created_at.clone()));
            context.set(
                "user.last_login",
                Value::string(user.last_login.as_deref().unwrap_or("-").to_string()),
            );
            if let Some(ref bio) = user.profile {
                context.set("user.bio", Value::string(bio.clone()));
            }

            let content = ctx.render_template("profile", &context)?;
            ctx.send(session, &content).await?;

            // Options
            ctx.send(
                session,
                &format!(
                    "[E]={} [P]={} [S]={} [Q]={}: ",
                    ctx.i18n.t("profile.edit"),
                    ctx.i18n.t("profile.change_password"),
                    ctx.i18n.t("menu.settings"),
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
                "s" => {
                    if let Some(result) = Self::change_settings(ctx, session, user_id).await? {
                        return Ok(result);
                    }
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
        let new_profile = match ctx.read_multiline(session).await? {
            Some(text) if !text.is_empty() => Some(Some(text)),
            Some(_) => None, // Empty input, no change
            None => return Ok(()), // Cancelled
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

    /// Change language, encoding, and terminal settings.
    async fn change_settings(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        user_id: i64,
    ) -> Result<Option<ScreenResult>> {
        // Get current settings
        let (current_language, current_encoding, current_terminal) = {
            let user_repo = UserRepository::new(&ctx.db);
            let user = match user_repo.get_by_id(user_id)? {
                Some(u) => u,
                None => return Ok(None),
            };
            (user.language.clone(), user.encoding, user.terminal.clone())
        };

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("menu.settings")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Show current settings
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("settings.language"),
                if current_language == "ja" {
                    "日本語"
                } else {
                    "English"
                }
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("settings.encoding"),
                current_encoding.as_str().to_uppercase()
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("settings.terminal_profile"),
                Self::profile_display_name(ctx, &current_terminal)
            ),
        )
        .await?;
        ctx.send_line(session, "").await?;

        // Language selection
        ctx.send_line(session, &format!("{}:", ctx.i18n.t("settings.language")))
            .await?;
        ctx.send_line(session, "  [1] English").await?;
        ctx.send_line(session, "  [2] 日本語 (Japanese)").await?;
        ctx.send(
            session,
            &format!(
                "{} [{}]: ",
                ctx.i18n.t("common.number"),
                if current_language == "ja" { "2" } else { "1" }
            ),
        )
        .await?;

        let lang_input = ctx.read_line(session).await?;
        let lang_input = lang_input.trim();

        let new_language = match lang_input {
            "1" => "en".to_string(),
            "2" => "ja".to_string(),
            "" => current_language.clone(),
            _ => current_language.clone(),
        };

        // Encoding selection
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("{}:", ctx.i18n.t("settings.encoding")))
            .await?;
        ctx.send_line(session, "  [1] UTF-8").await?;
        ctx.send_line(session, "  [2] ShiftJIS").await?;
        ctx.send(
            session,
            &format!(
                "{} [{}]: ",
                ctx.i18n.t("common.number"),
                if current_encoding == CharacterEncoding::ShiftJIS {
                    "2"
                } else {
                    "1"
                }
            ),
        )
        .await?;

        let enc_input = ctx.read_line(session).await?;
        let enc_input = enc_input.trim();

        let new_encoding = match enc_input {
            "1" => CharacterEncoding::Utf8,
            "2" => CharacterEncoding::ShiftJIS,
            "" => current_encoding,
            _ => current_encoding,
        };

        // Terminal profile selection
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("{}:", ctx.i18n.t("settings.terminal_profile")),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [1] {}", ctx.i18n.t("terminal.profile_standard")),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [2] {}", ctx.i18n.t("terminal.profile_c64")),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [3] {}", ctx.i18n.t("terminal.profile_c64_ansi")),
        )
        .await?;
        let current_profile_num = match current_terminal.as_str() {
            "c64" => "2",
            "c64_ansi" => "3",
            _ => "1",
        };
        ctx.send(
            session,
            &format!("{} [{}]: ", ctx.i18n.t("common.number"), current_profile_num),
        )
        .await?;

        let term_input = ctx.read_line(session).await?;
        let term_input = term_input.trim();

        let new_terminal = match term_input {
            "1" => Some("standard".to_string()),
            "2" => Some("c64".to_string()),
            "3" => Some("c64_ansi".to_string()),
            "" => None, // No change
            _ => None,
        };

        // Determine actual new terminal value
        let actual_new_terminal = new_terminal.clone().unwrap_or_else(|| current_terminal.clone());

        // Check if anything changed
        let terminal_changed = new_terminal.is_some() && actual_new_terminal != current_terminal;
        if new_language == current_language
            && new_encoding == current_encoding
            && !terminal_changed
        {
            ctx.send_line(session, "").await?;
            return Ok(None);
        }

        // Save to database
        let user_repo = UserRepository::new(&ctx.db);
        let mut update = UserUpdate::new()
            .language(new_language.clone())
            .encoding(new_encoding);

        if terminal_changed {
            update = update.terminal(actual_new_terminal.clone());
        }

        match user_repo.update(user_id, &update) {
            Ok(_) => {
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("settings.settings_saved"))
                    .await?;

                // Return SettingsChanged to signal session_handler to update
                Ok(Some(ScreenResult::SettingsChanged {
                    language: new_language,
                    encoding: new_encoding,
                    terminal_profile: if terminal_changed {
                        Some(actual_new_terminal)
                    } else {
                        None
                    },
                }))
            }
            Err(e) => {
                error!("Failed to save settings: {}", e);
                ctx.send_line(session, ctx.i18n.t("common.operation_failed"))
                    .await?;
                Ok(None)
            }
        }
    }

    /// Get display name for a terminal profile.
    fn profile_display_name(ctx: &ScreenContext, profile: &str) -> String {
        match profile {
            "c64" => ctx.i18n.t("terminal.profile_c64").to_string(),
            "c64_ansi" => ctx.i18n.t("terminal.profile_c64_ansi").to_string(),
            _ => ctx.i18n.t("terminal.profile_standard").to_string(),
        }
    }

    /// Get display name for a role.
    fn role_name(ctx: &ScreenContext, role: Role) -> String {
        match role {
            Role::Guest => ctx.i18n.t("role.guest").to_string(),
            Role::Member => ctx.i18n.t("role.member").to_string(),
            Role::SubOp => ctx.i18n.t("role.subop").to_string(),
            Role::SysOp => ctx.i18n.t("role.sysop").to_string(),
        }
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
