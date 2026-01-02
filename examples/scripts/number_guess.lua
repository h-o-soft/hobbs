-- @name Number Guess
-- @description Guess a number between 1-100. High score tracking included.
-- @author HOBBS Sample
-- @min_role 0

bbs.println("=== Number Guessing Game ===")
bbs.println("")
bbs.println("I'm thinking of a number between 1 and 100.")
bbs.println("Can you guess it?")
bbs.println("")

local answer = bbs.random(1, 100)
local attempts = 0
local max_attempts = 10

-- Load best score
local best = bbs.user_data.get("best_score")
if best then
    bbs.println("Your best score: " .. best .. " attempts")
else
    bbs.println("No best score yet!")
end
bbs.println("")

while attempts < max_attempts do
    local remaining = max_attempts - attempts
    bbs.println("Attempts remaining: " .. remaining)

    local guess = bbs.input_number("Your guess: ")

    if guess == nil then
        bbs.println("Please enter a number.")
    elseif guess < 1 or guess > 100 then
        bbs.println("Please enter a number between 1 and 100.")
    else
        attempts = attempts + 1

        if guess < answer then
            bbs.println("Too low!")
        elseif guess > answer then
            bbs.println("Too high!")
        else
            bbs.println("")
            bbs.println("*** Correct! ***")
            bbs.println("You got it in " .. attempts .. " attempts!")

            -- Update best score
            if not best or attempts < best then
                bbs.user_data.set("best_score", attempts)
                bbs.println("New high score!")
            end
            return
        end
    end
    bbs.println("")
end

bbs.println("")
bbs.println("Game over! The answer was " .. answer)
