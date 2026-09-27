//! Profile screen handler.

use tracing::error;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::auth::{change_password, update_profile, ProfileUpdateRequest};
use crate::datetime::format_datetime;
use crate::db::{Role, UserRepository, UserUpdate};
use crate::error::Result;
use crate::server::{EchoMode, TelnetSession};
use crate::template::Value;
use crate::terminal::settings::resolve;
use crate::terminal::TerminalProfile;

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
            let user_repo = UserRepository::new(ctx.db.pool());
            let user = match user_repo.get_by_id(user_id).await? {
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
            context.set(
                "user.role_name",
                Value::string(Self::role_name(ctx, user.role)),
            );
            context.set(
                "user.created_at",
                Value::string(format_datetime(
                    &user.created_at,
                    &ctx.config.server.timezone,
                    "%Y/%m/%d %H:%M",
                )),
            );
            context.set(
                "user.last_login",
                Value::string(
                    user.last_login
                        .as_deref()
                        .map(|dt| format_datetime(dt, &ctx.config.server.timezone, "%Y/%m/%d %H:%M"))
                        .unwrap_or_else(|| "-".to_string()),
                ),
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
            let user_repo = UserRepository::new(ctx.db.pool());
            let user = match user_repo.get_by_id(user_id).await? {
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
            Some(_) => None,       // Empty input, no change
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
        let user_repo = UserRepository::new(ctx.db.pool());
        match update_profile(&user_repo, user_id, request).await {
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
        let user_repo = UserRepository::new(ctx.db.pool());
        match change_password(&user_repo, user_id, &current, &new_password).await {
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

    /// Change settings: language, screen (width / custom profile) and
    /// auto-paging.
    ///
    /// The character encoding is decided by the connection type chosen when
    /// connecting, so it is shown but not changed here (plan.md D2/D3).
    async fn change_settings(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        user_id: i64,
    ) -> Result<Option<ScreenResult>> {
        // Get current settings
        let (current_language, current_terminal, current_auto_paging) = {
            let user_repo = UserRepository::new(ctx.db.pool());
            let user = match user_repo.get_by_id(user_id).await? {
                Some(u) => u,
                None => return Ok(None),
            };
            (
                user.language.clone(),
                user.terminal.clone(),
                user.auto_paging,
            )
        };
        let encoding = session.encoding();
        let choices = Self::screen_choices(ctx, session);
        // "Current" is the screen in effect for this connection, which may
        // differ from the saved one (login substitutes a profile that fits
        // the connection, e.g. a saved c64 on a PC connection).
        let current_choice =
            Self::current_screen_choice(ctx, &choices, &session.settings().profile.name);

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("menu.settings")))
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
                "{}: {} ({})",
                ctx.i18n.t("settings.encoding"),
                encoding.as_str().to_uppercase(),
                ctx.i18n.t("settings.encoding_from_connection")
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("settings.screen"),
                choices[current_choice].1
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "{}: {}",
                ctx.i18n.t("settings.auto_paging"),
                if current_auto_paging {
                    ctx.i18n.t("settings.enabled")
                } else {
                    ctx.i18n.t("settings.disabled")
                }
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
        let new_language = match lang_input.trim() {
            "1" => "en".to_string(),
            "2" => "ja".to_string(),
            _ => current_language.clone(),
        };

        // Screen selection (only when there is something to choose)
        let mut new_terminal: Option<String> = None;
        if choices.len() > 1 {
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("{}:", ctx.i18n.t("settings.screen")))
                .await?;
            for (i, (_, label)) in choices.iter().enumerate() {
                ctx.send_line(session, &format!("  [{}] {}", i + 1, label))
                    .await?;
            }
            ctx.send(
                session,
                &format!("{} [{}]: ", ctx.i18n.t("common.number"), current_choice + 1),
            )
            .await?;

            let screen_input = ctx.read_line(session).await?;
            if let Ok(idx) = screen_input.trim().parse::<usize>() {
                // An explicit selection is always applied, even when it is
                // the current one (codex review R2-F2 on #337).
                if idx >= 1 && idx <= choices.len() {
                    new_terminal = Some(choices[idx - 1].0.clone());
                }
            }
        }

        // Auto-paging selection
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("{}:", ctx.i18n.t("settings.auto_paging")))
            .await?;
        ctx.send_line(
            session,
            &format!("  [1] {}", ctx.i18n.t("settings.auto_paging_on")),
        )
        .await?;
        ctx.send_line(
            session,
            &format!("  [2] {}", ctx.i18n.t("settings.auto_paging_off")),
        )
        .await?;
        ctx.send(
            session,
            &format!(
                "{} [{}]: ",
                ctx.i18n.t("common.number"),
                if current_auto_paging { "1" } else { "2" }
            ),
        )
        .await?;

        let paging_input = ctx.read_line(session).await?;
        let new_auto_paging = match paging_input.trim() {
            "1" => true,
            "2" => false,
            _ => current_auto_paging,
        };

        // Check if anything changed
        let auto_paging_changed = new_auto_paging != current_auto_paging;
        if new_language == current_language && new_terminal.is_none() && !auto_paging_changed {
            ctx.send_line(session, "").await?;
            return Ok(None);
        }

        // Save to database
        let user_repo = UserRepository::new(ctx.db.pool());
        let mut update = UserUpdate::new().language(new_language.clone());
        if let Some(ref terminal) = new_terminal {
            if *terminal != current_terminal {
                update = update.terminal(terminal.clone());
            }
        }
        if auto_paging_changed {
            update = update.auto_paging(new_auto_paging);
        }

        match user_repo.update(user_id, &update).await {
            Ok(_) => {
                ctx.send_line(session, "").await?;
                ctx.send_line(session, ctx.i18n.t("settings.settings_saved"))
                    .await?;

                // Return SettingsChanged to signal session_handler to update
                Ok(Some(ScreenResult::SettingsChanged {
                    language: new_language,
                    terminal_profile: new_terminal,
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

    /// Screen choices for the settings screen on this connection.
    fn screen_choices(ctx: &ScreenContext, session: &TelnetSession) -> Vec<(String, String)> {
        Self::build_screen_choices(
            session.encoding(),
            &ctx.config.terminal.profiles,
            &session.settings().profile,
            |key| ctx.i18n.t(key).to_string(),
        )
    }

    /// Build the screen choices for a connection with `encoding`:
    /// (profile name to save, label).
    ///
    /// - ShiftJIS / UTF-8 connections offer 80 and 40 columns; CP437 and
    ///   PETSCII have a fixed screen. Custom profiles are added after them.
    /// - Every candidate is resolved the same way it is applied (custom
    ///   profiles first), so a custom profile overriding a built-in name is
    ///   labelled from its own size, listed once, and dropped when it does not
    ///   fit the encoding.
    /// - If nothing fits, `current` (the profile in effect) is offered.
    fn build_screen_choices(
        encoding: crate::server::CharacterEncoding,
        custom: &[crate::config::ProfileConfig],
        current: &TerminalProfile,
        label: impl Fn(&str) -> String,
    ) -> Vec<(String, String)> {
        use crate::server::CharacterEncoding as E;
        let builtin: &[(&str, &str)] = match encoding {
            E::ShiftJIS => &[
                ("standard", "settings.screen_80"),
                ("40col_sjis", "settings.screen_40"),
            ],
            E::Utf8 => &[
                ("standard_utf8", "settings.screen_80"),
                ("40col_utf8", "settings.screen_40"),
            ],
            E::Cp437 => &[("dos", "terminal.profile_dos")],
            E::Petscii => &[("c64", "terminal.profile_c64")],
        };
        let candidates = builtin
            .iter()
            .map(|&(name, key)| (name.to_string(), Some(key)))
            .chain(custom.iter().map(|c| (c.name.clone(), None)));

        let mut choices: Vec<(String, String)> = Vec::new();
        for (name, key) in candidates {
            if choices.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name)) {
                continue;
            }
            let profile = TerminalProfile::from_name_with_custom(&name, custom);
            if !resolve::profile_fits(&profile, encoding) {
                continue;
            }
            let overridden = custom.iter().any(|c| c.name.eq_ignore_ascii_case(&name));
            let text = match key {
                Some(key) if !overridden => label(key),
                _ => format!("{} ({}x{})", profile.name, profile.width, profile.height),
            };
            choices.push((name, text));
        }
        if choices.is_empty() {
            choices.push((
                current.name.clone(),
                format!("{} ({}x{})", current.name, current.width, current.height),
            ));
        }
        choices
    }

    /// Index of the saved terminal among `choices`: the same name, otherwise
    /// the first choice with the same width (e.g. a saved "40col_sjis" on a
    /// UTF-8 connection selects "40 columns"), otherwise the first.
    fn current_screen_choice(
        ctx: &ScreenContext,
        choices: &[(String, String)],
        current_terminal: &str,
    ) -> usize {
        if let Some(i) = choices
            .iter()
            .position(|(name, _)| name.eq_ignore_ascii_case(current_terminal))
        {
            return i;
        }
        let custom = &ctx.config.terminal.profiles;
        let width = TerminalProfile::from_name_with_custom(current_terminal, custom).width;
        choices
            .iter()
            .position(|(name, _)| {
                TerminalProfile::from_name_with_custom(name, custom).width == width
            })
            .unwrap_or(0)
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

    use crate::config::ProfileConfig;
    use crate::server::CharacterEncoding;

    fn custom(name: &str, width: u16, height: u16, encoding: &str) -> ProfileConfig {
        ProfileConfig {
            name: name.to_string(),
            width,
            height,
            cjk_width: 2,
            ansi_enabled: true,
            encoding: encoding.to_string(),
            output_mode: "ansi".to_string(),
            template_dir: "80".to_string(),
        }
    }

    fn choices(encoding: CharacterEncoding, customs: &[ProfileConfig]) -> Vec<(String, String)> {
        ProfileScreen::build_screen_choices(
            encoding,
            customs,
            &TerminalProfile::standard(),
            |key| format!("<{key}>"),
        )
    }

    #[test]
    fn test_screen_choices_builtin() {
        assert_eq!(
            choices(CharacterEncoding::ShiftJIS, &[]),
            vec![
                ("standard".to_string(), "<settings.screen_80>".to_string()),
                ("40col_sjis".to_string(), "<settings.screen_40>".to_string()),
            ]
        );
        assert_eq!(
            choices(CharacterEncoding::Petscii, &[]),
            vec![("c64".to_string(), "<terminal.profile_c64>".to_string())]
        );
    }

    #[test]
    fn test_screen_choices_custom_added_when_it_fits() {
        let c = choices(
            CharacterEncoding::Utf8,
            &[
                custom("pc98", 80, 25, "shiftjis"),
                custom("petty", 40, 25, "petscii"),
            ],
        );
        let names: Vec<&str> = c.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["standard_utf8", "40col_utf8", "pc98"]);
        assert_eq!(c[2].1, "pc98 (80x25)");
    }

    /// A custom profile overriding a built-in name is resolved, labelled from
    /// its own size, listed once, and dropped if it does not fit
    /// (codex review R1-F1 on #338).
    #[test]
    fn test_screen_choices_custom_overriding_builtin_name() {
        let c = choices(
            CharacterEncoding::ShiftJIS,
            &[custom("standard", 40, 25, "shiftjis")],
        );
        assert_eq!(
            c,
            vec![
                ("standard".to_string(), "standard (40x25)".to_string()),
                ("40col_sjis".to_string(), "<settings.screen_40>".to_string()),
            ]
        );
        // An override that does not fit the connection is not offered.
        let c = choices(
            CharacterEncoding::ShiftJIS,
            &[custom("standard", 80, 24, "petscii")],
        );
        let names: Vec<&str> = c.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["40col_sjis"]);
    }

    #[test]
    fn test_screen_choices_never_empty() {
        // Every candidate overridden by something that does not fit: the
        // profile in effect is offered so the screen always has a choice.
        let c = ProfileScreen::build_screen_choices(
            CharacterEncoding::Petscii,
            &[custom("c64", 80, 24, "shiftjis")],
            &TerminalProfile::c64(),
            |key| key.to_string(),
        );
        assert_eq!(c, vec![("c64".to_string(), "c64 (40x25)".to_string())]);
    }
}
