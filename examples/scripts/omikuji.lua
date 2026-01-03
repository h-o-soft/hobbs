-- @name Omikuji
-- @name.ja おみくじ
-- @description Daily fortune drawing. Can only draw once per day.
-- @description.ja 一日一回のおみくじ
-- @author HOBBS Sample
-- @min_role 0

bbs.println("=== Omikuji (Fortune) ===")
bbs.println("")

-- Check if already drawn today
local today = bbs.get_date()
local last_date = bbs.user_data.get("last_draw_date")

if last_date == today then
    local result = bbs.user_data.get("last_result")
    bbs.println("You already drew your fortune today!")
    bbs.println("")
    bbs.println("Today's result: " .. result)
    bbs.println("")
    bbs.println("Come back tomorrow for a new fortune!")
    return
end

-- Auto-draw mode (no input required)
bbs.println("Drawing your fortune...")
bbs.sleep(1)

-- Fortune weights (total = 100)
local fortunes = {
    {name = "Great Blessing", weight = 10, message = "Excellent luck! Everything will go your way!"},
    {name = "Blessing", weight = 20, message = "Good fortune ahead. Stay positive!"},
    {name = "Middle Blessing", weight = 25, message = "Average luck. Steady progress awaits."},
    {name = "Small Blessing", weight = 20, message = "Minor good fortune. Small joys ahead."},
    {name = "Future Blessing", weight = 15, message = "Patience will bring rewards."},
    {name = "Bad Luck", weight = 10, message = "Be cautious today. Tomorrow will be better."}
}

-- Weighted random selection
local total = 0
for _, f in ipairs(fortunes) do
    total = total + f.weight
end

local roll = bbs.random(1, total)
local cumulative = 0
local result = fortunes[1]

for _, f in ipairs(fortunes) do
    cumulative = cumulative + f.weight
    if roll <= cumulative then
        result = f
        break
    end
end

-- Display result
bbs.println("")
bbs.println("+-------------------+")
bbs.println("|                   |")
bbs.println("|   " .. result.name)
bbs.println("|                   |")
bbs.println("+-------------------+")
bbs.println("")
bbs.println(result.message)
bbs.println("")

-- Save result
bbs.user_data.set("last_draw_date", today)
bbs.user_data.set("last_result", result.name)

bbs.println("Come back tomorrow for a new fortune!")
