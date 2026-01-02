//! Member list screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::db::UserRepository;
use crate::error::Result;
use crate::server::TelnetSession;

/// Member list screen handler.
pub struct MemberScreen;

impl MemberScreen {
    /// Run the member list screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        loop {
            // Display member list header
            ctx.send_line(session, "").await?;
            ctx.send_line(session, &format!("=== {} ===", ctx.i18n.t("member.list")))
                .await?;
            ctx.send_line(session, "").await?;

            // Get member list from database
            let user_repo = UserRepository::new(&ctx.db);
            let users = user_repo.list_active()?;

            if users.is_empty() {
                ctx.send_line(session, ctx.i18n.t("member.no_members"))
                    .await?;
            } else {
                // Header
                ctx.send_line(
                    session,
                    &format!(
                        "  {:<16} {:<20} {:<10}",
                        ctx.i18n.t("member.username"),
                        ctx.i18n.t("member.nickname"),
                        ctx.i18n.t("member.role")
                    ),
                )
                .await?;
                ctx.send_line(session, &"-".repeat(50)).await?;

                // List users
                for user in &users {
                    let role_str = match user.role {
                        crate::db::Role::Guest => ctx.i18n.t("role.guest"),
                        crate::db::Role::Member => ctx.i18n.t("role.member"),
                        crate::db::Role::SubOp => ctx.i18n.t("role.subop"),
                        crate::db::Role::SysOp => ctx.i18n.t("role.sysop"),
                    };
                    ctx.send_line(
                        session,
                        &format!(
                            "  {:<16} {:<20} {:<10}",
                            user.username, user.nickname, role_str
                        ),
                    )
                    .await?;
                }

                ctx.send_line(session, "").await?;
                ctx.send_line(
                    session,
                    &ctx.i18n
                        .t_with("member.total", &[("count", &users.len().to_string())]),
                )
                .await?;
            }

            ctx.send_line(session, "").await?;
            ctx.send(session, &format!("[Q={}]: ", ctx.i18n.t("common.back")))
                .await?;

            let input = ctx.read_line(session).await?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") || input.is_empty() {
                return Ok(ScreenResult::Back);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_screen_exists() {
        let _ = MemberScreen;
    }
}
