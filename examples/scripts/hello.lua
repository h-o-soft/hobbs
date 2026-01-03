-- @name Hello World
-- @name.ja ハローワールド
-- @description Simple greeting script that demonstrates BBS API
-- @description.ja BBS APIのデモンストレーション
-- @author HOBBS Sample
-- @min_role 0

-- Get user information
local user = bbs.get_user()

bbs.println("=== " .. bbs.t("title") .. " ===")
bbs.println("")
bbs.println(string.format(bbs.t("welcome"), user.nickname))
bbs.println("")
bbs.println(bbs.t("user_info"))
bbs.println(string.format(bbs.t("username"), user.username))
bbs.println(string.format(bbs.t("role"), user.role))
if bbs.is_guest() then
    bbs.println(string.format(bbs.t("status"), bbs.t("guest")))
else
    bbs.println(string.format(bbs.t("status"), bbs.t("registered")))
end
bbs.println("")
bbs.println(bbs.t("terminal_info"))
bbs.println(string.format(bbs.t("width"), bbs.terminal.width))
bbs.println(string.format(bbs.t("height"), bbs.terminal.height))
bbs.println(string.format(bbs.t("ansi"), tostring(bbs.terminal.has_ansi)))
bbs.println("")
bbs.println(bbs.t("date_time"))
bbs.println(string.format(bbs.t("date"), bbs.get_date()))
bbs.println(string.format(bbs.t("time"), bbs.get_time()))
bbs.println("")
bbs.println(string.format(bbs.t("random"), bbs.random(1, 100)))
bbs.println("")
bbs.println(bbs.t("thanks"))
