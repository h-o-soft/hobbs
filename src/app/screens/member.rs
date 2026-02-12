//! Member list screen handler.

use super::common::ScreenContext;
use super::ScreenResult;
use crate::db::UserRepository;
use crate::error::Result;
use crate::server::TelnetSession;
use crate::template::Value;

/// Member list screen handler.
pub struct MemberScreen;

impl MemberScreen {
    /// Run the member list screen.
    pub async fn run(ctx: &mut ScreenContext, session: &mut TelnetSession) -> Result<ScreenResult> {
        loop {
            // Display member list using template
            let user_repo = UserRepository::new(ctx.db.pool());
            let users = user_repo.list_active().await?;

            let mut context = ctx.create_context();
            context.set("has_members", Value::bool(!users.is_empty()));

            if !users.is_empty() {
                let mut member_list = Vec::new();
                for user in &users {
                    let role_str = match user.role {
                        crate::db::Role::Guest => ctx.i18n.t("role.guest"),
                        crate::db::Role::Member => ctx.i18n.t("role.member"),
                        crate::db::Role::SubOp => ctx.i18n.t("role.subop"),
                        crate::db::Role::SysOp => ctx.i18n.t("role.sysop"),
                    };
                    let mut entry = std::collections::HashMap::new();
                    entry.insert("username".to_string(), Value::string(&user.username));
                    entry.insert("nickname".to_string(), Value::string(&user.nickname));
                    entry.insert("role".to_string(), Value::string(role_str));
                    member_list.push(Value::Object(entry));
                }
                context.set("members", Value::List(member_list));
                context.set("total_text", Value::string(
                    ctx.i18n.t_with("member.total", &[("count", &users.len().to_string())]).to_string(),
                ));
            }

            let content = ctx.render_template("member/list", &context)?;
            ctx.send(session, &content).await?;
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
