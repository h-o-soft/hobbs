-- @name Daily Fortune
-- @name.ja 今日の運勢
-- @description Get a random fortune message
-- @description.ja ランダムな運勢メッセージ
-- @author HOBBS Sample
-- @min_role 0

local user = bbs.get_user()

bbs.println("=== Daily Fortune ===")
bbs.println("")
bbs.println("Welcome, " .. user.nickname .. "!")
bbs.println("")

local fortunes = {
    "Today is your lucky day! Take a chance on something new.",
    "A pleasant surprise awaits you around the corner.",
    "Your hard work will soon pay off. Keep going!",
    "Someone is thinking of you right now.",
    "A great opportunity is coming your way. Be ready!",
    "Trust your instincts today - they won't lead you astray.",
    "Good things come to those who wait... and work for it!",
    "Your creativity will shine today. Express yourself!",
    "A new friendship may blossom unexpectedly.",
    "Take time to appreciate the small things today.",
    "Your positive attitude will attract positive results.",
    "An old friend may reach out to you soon.",
    "Today is perfect for starting something new.",
    "Your kindness will be rewarded in unexpected ways.",
    "Stay curious - there's always more to learn.",
    "A challenge today will become tomorrow's success story.",
    "Your patience will be tested, but rewarded.",
    "Share your knowledge with others - it multiplies!",
    "Take a moment to relax and recharge.",
    "Adventure awaits those who seek it!"
}

local lucky_numbers = {}
for i = 1, 3 do
    table.insert(lucky_numbers, bbs.random(1, 99))
end

local colors = {"Red", "Blue", "Green", "Yellow", "Purple", "Orange", "Pink", "Gold"}
local lucky_color = colors[bbs.random(1, #colors)]

bbs.println("Consulting the stars...")
bbs.sleep(1)
bbs.println("")

local fortune = fortunes[bbs.random(1, #fortunes)]

bbs.println("+----------------------------------------+")
bbs.println("|                                        |")
bbs.println("|  Your fortune for today:               |")
bbs.println("|                                        |")
bbs.println("+----------------------------------------+")
bbs.println("")
bbs.println(fortune)
bbs.println("")
bbs.println("Lucky numbers: " .. table.concat(lucky_numbers, ", "))
bbs.println("Lucky color: " .. lucky_color)
bbs.println("")
bbs.println("Time: " .. bbs.get_time())
bbs.println("Date: " .. bbs.get_date())
