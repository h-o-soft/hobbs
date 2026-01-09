//! Script screen handler.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::common::ScreenContext;
use super::ScreenResult;
use crate::db::{Role, UserRepository};
use crate::error::Result;
use crate::script::{create_script_runtime, Script, ScriptContext, ScriptMessage, ScriptService};
use crate::server::TelnetSession;

/// Simple result for script execution in the screen context.
struct ExecutionResult {
    success: bool,
    error: Option<String>,
}

/// Script screen handler.
pub struct ScriptScreen;

impl ScriptScreen {
    /// Run the script screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        let scripts_dir = Self::get_scripts_dir(ctx);
        let user_role = Self::get_user_role(ctx, session).await;

        loop {
            // Display script list
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("script.title")))
                .await?;
            ctx.send_line(session, "").await?;

            let service = ScriptService::new(ctx.db.pool(), &ctx.db).with_scripts_dir(&scripts_dir);
            let scripts = service.list_scripts(user_role).await?;

            // Display scripts with localized names/descriptions
            let lang = ctx.i18n.locale();
            if scripts.is_empty() {
                ctx.send_line(session, &ctx.i18n.t("script.no_scripts"))
                    .await?;
            } else {
                for (i, script) in scripts.iter().enumerate() {
                    let name = script.get_name(lang);
                    let description = script.get_description(lang).unwrap_or("");
                    let line = format!(
                        "  [{:>2}] {}{}",
                        i + 1,
                        name,
                        if description.is_empty() {
                            String::new()
                        } else {
                            format!(" - {}", description)
                        }
                    );
                    ctx.send_line(session, &line).await?;
                }
            }

            ctx.send_line(session, "").await?;

            // Show admin options for SubOp/SysOp
            if user_role >= 2 {
                ctx.send_line(session, &format!("  [A] {}", ctx.i18n.t("script.admin")))
                    .await?;
                ctx.send_line(session, "").await?;
            }

            // If no scripts and not admin, just go back
            if scripts.is_empty() && user_role < 2 {
                ctx.wait_for_enter(session).await?;
                return Ok(ScreenResult::Back);
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
                "a" if user_role >= 2 => {
                    Self::admin_menu(ctx, session, &scripts_dir).await?;
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
        let service = ScriptService::new(ctx.db.pool(), &ctx.db).with_scripts_dir(scripts_dir);

        let script = match service.get_script_by_id(script_id).await? {
            Some(s) => s,
            None => {
                ctx.send_line(session, &ctx.i18n.t("script.not_found"))
                    .await?;
                return Ok(());
            }
        };

        // Create script context
        let script_context = Self::create_script_context(ctx, session).await;

        // Display script header with localized name
        let lang = ctx.i18n.locale();
        let name = script.get_name(lang);

        ctx.send_line(session, "").await?;
        ctx.send_line(session, &format!("--- {} ---", name)).await?;
        ctx.send_line(session, "").await?;

        // Execute script with interactive message loop
        let result =
            Self::execute_with_message_loop(ctx, session, scripts_dir, &script, script_context)
                .await?;

        if !result.success {
            ctx.send_line(session, "").await?;
            if let Some(error) = &result.error {
                ctx.send_line(
                    session,
                    &format!("{}: {}", ctx.i18n.t("script.error"), error),
                )
                .await?;
            }
        }

        ctx.send_line(session, "").await?;
        ctx.send_line(session, "---").await?;
        ctx.wait_for_enter(session).await?;

        Ok(())
    }

    /// Execute a script with an async message loop for I/O.
    ///
    /// This runs the Lua script in a blocking thread while handling
    /// output and input requests asynchronously through message passing.
    async fn execute_with_message_loop(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        scripts_dir: &PathBuf,
        script: &Script,
        script_context: ScriptContext,
    ) -> Result<ExecutionResult> {
        // Create runtime and handle for message passing
        let (runtime, handle) = create_script_runtime();
        let handle = Arc::new(handle);

        // Clone data needed for the blocking task
        let db = Arc::clone(&ctx.db);
        let scripts_dir = scripts_dir.clone();
        let script_clone = script.clone();

        // Spawn the script execution in a blocking thread
        let script_handle = Arc::clone(&handle);
        let task_handle = tokio::task::spawn_blocking(move || {
            let service = ScriptService::new(db.pool(), &db).with_scripts_dir(&scripts_dir);

            // Execute with runtime for interactive I/O
            match service.execute_with_runtime(&script_clone, script_context, Some(script_handle)) {
                Ok(result) => ExecutionResult {
                    success: result.success,
                    error: result.error,
                },
                Err(e) => ExecutionResult {
                    success: false,
                    error: Some(e.to_string()),
                },
            }
        });

        // Message loop: handle output and input requests
        let result = loop {
            // Poll for messages with a timeout
            match runtime.recv_timeout(Duration::from_millis(50)) {
                Some(ScriptMessage::Output(text)) => {
                    // Send output to the terminal
                    ctx.send(session, &text).await?;
                }
                Some(ScriptMessage::InputRequest { prompt }) => {
                    // Display prompt if provided
                    if let Some(p) = prompt {
                        ctx.send(session, &p).await?;
                    }

                    // Read input from the user
                    let input = ctx.read_line(session).await?;

                    // Send the response back to the script
                    runtime.send_input(Some(input));
                }
                Some(ScriptMessage::Done { success, error }) => {
                    // Script finished
                    break ExecutionResult { success, error };
                }
                None => {
                    // Timeout - check if the task is still running
                    if task_handle.is_finished() {
                        // Task finished without sending Done message
                        // This might happen if the script panicked
                        match task_handle.await {
                            Ok(result) => break result,
                            Err(e) => {
                                break ExecutionResult {
                                    success: false,
                                    error: Some(format!("Script execution failed: {}", e)),
                                };
                            }
                        }
                    }
                }
            }
        };

        Ok(result)
    }

    /// Admin menu for SubOp/SysOp.
    async fn admin_menu(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        scripts_dir: &PathBuf,
    ) -> Result<()> {
        loop {
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("=== {} ===", ctx.i18n.t("script.admin_title")),
            )
            .await?;
            ctx.send_line(session, "").await?;
            ctx.send_line(
                session,
                &format!("  [1] {}", ctx.i18n.t("script.admin_resync")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [2] {}", ctx.i18n.t("script.admin_toggle")),
            )
            .await?;
            ctx.send_line(
                session,
                &format!("  [3] {}", ctx.i18n.t("script.admin_guide")),
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

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            match input.to_ascii_lowercase().as_str() {
                "q" | "" => return Ok(()),
                "1" => Self::resync_scripts(ctx, session, scripts_dir).await?,
                "2" => Self::toggle_scripts(ctx, session, scripts_dir).await?,
                "3" => Self::show_guide(ctx, session).await?,
                _ => {}
            }
        }
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

        let service = ScriptService::new(ctx.db.pool(), &ctx.db).with_scripts_dir(scripts_dir);
        let result = service.sync_scripts().await?;

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

    /// Toggle script enabled/disabled status.
    async fn toggle_scripts(
        ctx: &mut ScreenContext,
        session: &mut TelnetSession,
        scripts_dir: &PathBuf,
    ) -> Result<()> {
        // Get scripts first, then drop service to release borrow
        let scripts = {
            let service = ScriptService::new(ctx.db.pool(), &ctx.db).with_scripts_dir(scripts_dir);
            service.list_all_scripts().await?
        };

        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("script.toggle_title")),
        )
        .await?;
        ctx.send_line(session, "").await?;

        if scripts.is_empty() {
            ctx.send_line(session, &ctx.i18n.t("script.no_scripts"))
                .await?;
            ctx.wait_for_enter(session).await?;
            return Ok(());
        }

        // Display all scripts with enabled/disabled status
        let lang = ctx.i18n.locale().to_string();
        for (i, script) in scripts.iter().enumerate() {
            let name = script.get_name(&lang);
            let status = if script.enabled {
                ctx.i18n.t("script.toggle_enabled")
            } else {
                ctx.i18n.t("script.toggle_disabled")
            };
            ctx.send_line(session, &format!("  [{:>2}] [{}] {}", i + 1, status, name))
                .await?;
        }

        ctx.send_line(session, "").await?;
        ctx.send(
            session,
            &format!(
                "{} [Q={}]: ",
                ctx.i18n.t("script.toggle_prompt"),
                ctx.i18n.t("common.back")
            ),
        )
        .await?;

        let input = ctx.read_line(session).await?;
        let input = input.trim();

        if input.to_ascii_lowercase() == "q" || input.is_empty() {
            return Ok(());
        }

        if let Ok(num) = input.parse::<usize>() {
            if num > 0 && num <= scripts.len() {
                let script = &scripts[num - 1];
                let new_enabled = !script.enabled;

                // Create new service for set_enabled
                let service = ScriptService::new(ctx.db.pool(), &ctx.db).with_scripts_dir(scripts_dir);
                service.set_enabled(script.id, new_enabled).await?;

                let name = script.get_name(&lang);
                let message = if new_enabled {
                    ctx.i18n.t_with("script.toggled_on", &[("name", &name)])
                } else {
                    ctx.i18n.t_with("script.toggled_off", &[("name", &name)])
                };
                ctx.send_line(session, "").await?;
                ctx.send_line(session, &message).await?;
                ctx.wait_for_enter(session).await?;
            }
        }

        Ok(())
    }

    /// Show script placement guide.
    async fn show_guide(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<()> {
        ctx.send_line(session, "").await?;
        ctx.send_line(
            session,
            &format!("=== {} ===", ctx.i18n.t("script.guide_title")),
        )
        .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &ctx.i18n.t("script.guide_step1"))
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &ctx.i18n.t("script.guide_step2"))
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &ctx.i18n.t("script.guide_metadata"))
            .await?;
        ctx.send_line(session, "").await?;
        ctx.send_line(session, &ctx.i18n.t("script.guide_step3"))
            .await?;
        ctx.send_line(session, &ctx.i18n.t("script.guide_step4"))
            .await?;
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
    async fn get_user_role(ctx: &ScreenContext, session: &TelnetSession) -> i32 {
        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(ctx.db.pool());
            if let Ok(Some(user)) = user_repo.get_by_id(user_id).await {
                return user.role as i32;
            }
        }
        Role::Guest as i32
    }

    /// Create a script execution context from the session.
    async fn create_script_context(ctx: &ScreenContext, session: &TelnetSession) -> ScriptContext {
        let lang = ctx.i18n.locale().to_string();

        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(ctx.db.pool());
            if let Ok(Some(user)) = user_repo.get_by_id(user_id).await {
                return ScriptContext {
                    script_id: None, // Set by ScriptService.execute()
                    user_id: Some(user_id),
                    username: user.username,
                    nickname: user.nickname,
                    user_role: user.role as i32,
                    terminal_width: ctx.profile.width,
                    terminal_height: ctx.profile.height,
                    has_ansi: ctx.profile.ansi_enabled,
                    lang,
                    translations: std::collections::HashMap::new(), // Set by ScriptService
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
            lang,
            translations: std::collections::HashMap::new(), // Set by ScriptService
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
