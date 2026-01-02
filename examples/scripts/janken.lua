-- @name Janken
-- @description Rock-Paper-Scissors game with win/loss tracking
-- @author HOBBS Sample
-- @min_role 0

local user = bbs.get_user()
bbs.println("=== Rock-Paper-Scissors ===")
bbs.println("")
bbs.println("Hello, " .. user.nickname .. "!")
bbs.println("")

-- Load stats
local wins = bbs.user_data.get("wins") or 0
local losses = bbs.user_data.get("losses") or 0
local draws = bbs.user_data.get("draws") or 0

bbs.println("Your record: " .. wins .. "W " .. losses .. "L " .. draws .. "D")
bbs.println("")

local hands = {"Rock", "Scissors", "Paper"}

while true do
    bbs.println("[1] Rock  [2] Scissors  [3] Paper  [Q] Quit")
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
        bbs.println("You: " .. hands[player])
        bbs.println("CPU: " .. hands[cpu])

        if player == cpu then
            bbs.println("Draw!")
            draws = draws + 1
            bbs.user_data.set("draws", draws)
        elseif (player == 1 and cpu == 2) or
               (player == 2 and cpu == 3) or
               (player == 3 and cpu == 1) then
            bbs.println("You win!")
            wins = wins + 1
            bbs.user_data.set("wins", wins)
        else
            bbs.println("You lose...")
            losses = losses + 1
            bbs.user_data.set("losses", losses)
        end
        bbs.println("")
    else
        bbs.println("Please enter 1, 2, 3, or Q")
        bbs.println("")
    end
end

bbs.println("")
bbs.println("Final record: " .. wins .. "W " .. losses .. "L " .. draws .. "D")
bbs.println("See you next time!")
