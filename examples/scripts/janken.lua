-- @name Janken
-- @name.ja じゃんけん
-- @description Rock-Paper-Scissors game with win/loss tracking
-- @description.ja じゃんけんゲーム（勝敗記録付き）
-- @author HOBBS Sample
-- @min_role 0

local user = bbs.get_user()
bbs.println("=== " .. bbs.t("title") .. " ===")
bbs.println("")
bbs.println(string.format(bbs.t("hello"), user.nickname))
bbs.println("")

-- Load stats
local wins = bbs.user_data.get("wins") or 0
local losses = bbs.user_data.get("losses") or 0
local draws = bbs.user_data.get("draws") or 0

bbs.println(string.format(bbs.t("record"), wins, losses, draws))
bbs.println("")

local hands = {bbs.t("rock"), bbs.t("scissors"), bbs.t("paper")}

while true do
    bbs.println(bbs.t("prompt"))
    local choice = bbs.input("> ")

    if choice == nil then
        break
    end

    choice = choice:upper()

    if choice == "Q" or choice == "" then
        break
    end

    local player = tonumber(choice)
    if player and player >= 1 and player <= 3 then
        local cpu = bbs.random(1, 3)

        bbs.println("")
        bbs.println(bbs.t("you") .. ": " .. hands[player])
        bbs.println(bbs.t("cpu") .. ": " .. hands[cpu])

        if player == cpu then
            bbs.println(bbs.t("draw"))
            draws = draws + 1
            bbs.user_data.set("draws", draws)
        elseif (player == 1 and cpu == 2) or
               (player == 2 and cpu == 3) or
               (player == 3 and cpu == 1) then
            bbs.println(bbs.t("you_win"))
            wins = wins + 1
            bbs.user_data.set("wins", wins)
        else
            bbs.println(bbs.t("you_lose"))
            losses = losses + 1
            bbs.user_data.set("losses", losses)
        end
        bbs.println("")
    else
        bbs.println(bbs.t("invalid"))
        bbs.println("")
    end
end

bbs.println("")
bbs.println(string.format(bbs.t("final_record"), wins, losses, draws))
bbs.println(bbs.t("goodbye"))
