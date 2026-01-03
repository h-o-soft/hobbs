-- @name Hello World
-- @name.ja ハローワールド
-- @description Simple greeting script that demonstrates BBS API
-- @description.ja BBS APIのデモンストレーション
-- @author HOBBS Sample
-- @min_role 0

-- Get user information
local user = bbs.get_user()

bbs.println("=== Hello World ===")
bbs.println("")
bbs.println("Welcome, " .. user.nickname .. "!")
bbs.println("")
bbs.println("User Information:")
bbs.println("  Username: " .. user.username)
bbs.println("  Role: " .. user.role)
if bbs.is_guest() then
    bbs.println("  Status: Guest")
else
    bbs.println("  Status: Registered User")
end
bbs.println("")
bbs.println("Terminal Information:")
bbs.println("  Width: " .. bbs.terminal.width)
bbs.println("  Height: " .. bbs.terminal.height)
bbs.println("  ANSI Support: " .. tostring(bbs.terminal.has_ansi))
bbs.println("")
bbs.println("Current Date/Time:")
bbs.println("  Date: " .. bbs.get_date())
bbs.println("  Time: " .. bbs.get_time())
bbs.println("")
bbs.println("Random number (1-100): " .. bbs.random(1, 100))
bbs.println("")
bbs.println("Thanks for using HOBBS!")
