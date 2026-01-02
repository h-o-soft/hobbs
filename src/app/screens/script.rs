//! Script screen handler.

use std::path::PathBuf;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::db::{Role, UserRepository};
use crate::error::Result;
use crate::script::{ScriptContext, ScriptService};
use crate::server::TelnetSession;

/// Script screen handler.
pub struct ScriptScreen;

impl ScriptScreen {
    /// Run the script screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        let scripts_dir = Self::get_scripts_dir(ctx);
        let user_role = Self::get_user_role(ctx, session);

        loop {
            // Display script list
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("script.title")))
                .await?;
            ctx.send_line(session, "").await?;

            let service = ScriptService::new(&ctx.db).with_scripts_dir(&scripts_dir);
            let scripts = service.list_scripts(user_role)?;

            if scripts.is_empty() {
                ctx.send_line(session, &ctx.i18n.t("script.no_scripts"))
                    .await?;
                ctx.send_line(session, "").await?;
                ctx.wait_for_enter(session).await?;
                return Ok(ScreenResult::Back);
            }

            // Display scripts
            for (i, script) in scripts.iter().enumerate() {
                let description = script.description.as_deref().unwrap_or("");
                let line = format!(
                    "  [{:>2}] {}{}",
                    i + 1,
                    script.name,
                    if description.is_empty() {
                        String::new()
                    } else {
                        format!(" - {}", description)
                    }
                );
                ctx.send_line(session, &line).await?;
            }

            ctx.send_line(session, "").await?;

            // Show admin options for SubOp/SysOp
            if user_role >= 2 {
                ctx.send_line(session, &format!("  [R] {}", ctx.i18n.t("script.resync")))
                    .await?;
            }

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
                "r" if user_role >= 2 => {
                    Self::resync_scripts(ctx, session, &scripts_dir).await?;
                }
                _ => {
                    // Try to parse as script number
                    if let Ok(num) = input.parse::<usize>() {
                        if num > 0 && num <= scripts.len() {
                            let script = &scripts[num - 1];
                            Self::execute_script(ctx, session, &scripts_dir, script.id).await?;
                        }
                    }
                }
            }
        }
    }

    /// Execute a script.
    async fn execute_script(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        scripts_dir: &PathBuf,
        script_id: i64,
    ) -> Result<()> {
        let service = ScriptService::new(&ctx.db).with_scripts_dir(scripts_dir);

        let script = match service.get_script_by_id(script_id)? {
            Some(s) => s,
            None => {
                ctx.send_line(session, &ctx.i18n.t("script.not_found"))
                    .await?;
                return Ok(());
            }
        };

        // Create script context
        let script_context = Self::create_script_context(ctx, session);

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("--- {} ---", script.name))
            .await?;
        ctx.send_line(session, "").await?;

        // Execute the script
        let result = service.execute(&script, script_context)?;

        // Display output
        for line in &result.output {
            ctx.send(session, line).await?;
        }

        if !result.success {
            ctx.send_line(session, "").await?;
            if let Some(error) = &result.error {
                ctx.send_line(
                    session,
                    &format!("{}:{}", ctx.i18n.t("script.error"), error),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(session, "---").await?;
        ctx.wait_for_enter(session).await?;

        Ok(())
    }

    /// Resync scripts from file system.
    async fn resync_scripts(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        scripts_dir: &PathBuf,
    ) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &ctx.i18n.t("script.syncing"))
            .await?;

        let service = ScriptService::new(&ctx.db).with_scripts_dir(scripts_dir);
        let result = service.sync_scripts()?;

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("  {}: {}", ctx.i18n.t("script.sync_added"), result.added),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "  {}: {}",
                ctx.i18n.t("script.sync_updated"),
                result.updated
            ),
        )
        .await?;
        ctx.send_line(
            session,
            &format!(
                "  {}: {}",
                ctx.i18n.t("script.sync_removed"),
                result.removed
            ),
        )
        .await?;

        if !result.errors.is_empty() {
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &ctx.i18n.t("script.sync_errors"))
                .await?;
            for (path, error) in &result.errors {
                ctx.send_line(session, &format!("    {}: {}", path, error))
                    .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.wait_for_enter(session).await?;

        Ok(())
    }

    /// Get the scripts directory path.
    fn get_scripts_dir(ctx: &ScreenContext) -> PathBuf {
        // Use config.files.storage_path as base, with scripts subdirectory
        let base = PathBuf::from(&ctx.config.files.storage_path);
        base.join("scripts")
    }

    /// Get the user's role from the session.
    fn get_user_role(ctx: &ScreenContext, session: &TelnetSession) -> i32 {
        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(&ctx.db);
            if let Ok(Some(user)) = user_repo.get_by_id(user_id) {
                return user.role as i32;
            }
        }
        Role::Guest as i32
    }

    /// Create a script execution context from the session.
    fn create_script_context(ctx: &ScreenContext, session: &TelnetSession) -> ScriptContext {
        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(&ctx.db);
            if let Ok(Some(user)) = user_repo.get_by_id(user_id) {
                return ScriptContext {
                    script_id: None, // Set by ScriptService.execute()
                    user_id: Some(user_id),
                    username: user.username,
                    nickname: user.nickname,
                    user_role: user.role as i32,
                    terminal_width: ctx.profile.width,
                    terminal_height: ctx.profile.height,
                    has_ansi: ctx.profile.ansi_enabled,
                };
            }
        }

        // Guest user
        ScriptContext {
            script_id: None, // Set by ScriptService.execute()
            user_id: None,
            username: "guest".to_string(),
            nickname: "Guest".to_string(),
            user_role: Role::Guest as i32,
            terminal_width: ctx.profile.width,
            terminal_height: ctx.profile.height,
            has_ansi: ctx.profile.ansi_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_screen_exists() {
        let _ = ScriptScreen;
    }
}
