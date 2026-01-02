-- @name Dice Roller
-- @description Roll dice with customizable sides
-- @author HOBBS Sample
-- @min_role 0

bbs.println("=== Dice Roller ===")
bbs.println("")
bbs.println("Roll various dice!")
bbs.println("")

while true do
    bbs.println("Dice options:")
    bbs.println("  [1] D6  (6-sided)")
    bbs.println("  [2] D20 (20-sided)")
    bbs.println("  [3] D100 (percentile)")
    bbs.println("  [4] Custom")
    bbs.println("  [Q] Quit")
    bbs.println("")

    local choice = bbs.input("> ")

    if choice == nil or choice:upper() == "Q" or choice == "" then
        break
    end

    local sides = nil
    local num = tonumber(choice)

    if num == 1 then
        sides = 6
    elseif num == 2 then
        sides = 20
    elseif num == 3 then
        sides = 100
    elseif num == 4 then
        bbs.println("Enter number of sides (2-1000):")
        sides = bbs.input_number("> ")
        if sides then
            sides = math.floor(sides)
            if sides < 2 then sides = 2 end
            if sides > 1000 then sides = 1000 end
        end
    end

    if sides then
        bbs.println("")
        bbs.println("Rolling D" .. sides .. "...")
        bbs.sleep(0.5)

        local result = bbs.random(1, sides)

        bbs.println("")
        bbs.println("  +-------+")
        bbs.println("  |       |")
        bbs.println("  |  " .. string.format("%3d", result) .. "  |")
        bbs.println("  |       |")
        bbs.println("  +-------+")
        bbs.println("")

        if sides == 20 then
            if result == 20 then
                bbs.println("*** CRITICAL! ***")
            elseif result == 1 then
                bbs.println("Critical fail...")
            end
            bbs.println("")
        end
    else
        bbs.println("Invalid choice.")
        bbs.println("")
    end
end

bbs.println("Thanks for rolling!")
