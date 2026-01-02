-- @name Quiz Game
-- @description Test your knowledge with trivia questions
-- @author HOBBS Sample
-- @min_role 0

local user = bbs.get_user()

bbs.println("=== Quiz Game ===")
bbs.println("")
bbs.println("Welcome, " .. user.nickname .. "!")
bbs.println("")

-- Load high score
local high_score = bbs.user_data.get("high_score") or 0
bbs.println("High score: " .. high_score .. " points")
bbs.println("")

local questions = {
    {
        q = "What is the capital of Japan?",
        choices = {"Osaka", "Tokyo", "Kyoto", "Nagoya"},
        answer = 2
    },
    {
        q = "How many planets are in our solar system?",
        choices = {"7", "8", "9", "10"},
        answer = 2
    },
    {
        q = "What year did World War II end?",
        choices = {"1943", "1944", "1945", "1946"},
        answer = 3
    },
    {
        q = "What is the largest mammal?",
        choices = {"Elephant", "Blue Whale", "Giraffe", "Polar Bear"},
        answer = 2
    },
    {
        q = "Which element has the symbol 'O'?",
        choices = {"Gold", "Oxygen", "Osmium", "Oganesson"},
        answer = 2
    },
    {
        q = "How many sides does a hexagon have?",
        choices = {"5", "6", "7", "8"},
        answer = 2
    },
    {
        q = "What is the speed of light (approx)?",
        choices = {"100,000 km/s", "200,000 km/s", "300,000 km/s", "400,000 km/s"},
        answer = 3
    },
    {
        q = "Which programming language is named after a snake?",
        choices = {"Cobra", "Python", "Viper", "Anaconda"},
        answer = 2
    }
}

-- Shuffle and pick 5 questions
local selected = {}
local indices = {}
for i = 1, #questions do
    table.insert(indices, i)
end

for i = 1, math.min(5, #questions) do
    local j = bbs.random(i, #indices)
    indices[i], indices[j] = indices[j], indices[i]
    table.insert(selected, questions[indices[i]])
end

local score = 0
local total = #selected

bbs.println("Answer " .. total .. " questions!")
bbs.println("Press Enter to start...")
bbs.pause()

for i, q in ipairs(selected) do
    bbs.clear()
    bbs.println("=== Question " .. i .. "/" .. total .. " ===")
    bbs.println("")
    bbs.println(q.q)
    bbs.println("")

    for j, choice in ipairs(q.choices) do
        bbs.println("  [" .. j .. "] " .. choice)
    end
    bbs.println("")

    local answer = bbs.input_number("Your answer: ")

    if answer and math.floor(answer) == q.answer then
        bbs.println("")
        bbs.println("Correct!")
        score = score + 1
    else
        bbs.println("")
        bbs.println("Wrong! The answer was: " .. q.choices[q.answer])
    end

    bbs.sleep(1)
end

bbs.clear()
bbs.println("=== Results ===")
bbs.println("")
bbs.println("Score: " .. score .. "/" .. total)
bbs.println("")

local percent = math.floor(score / total * 100)
if percent >= 80 then
    bbs.println("Excellent! You're a quiz master!")
elseif percent >= 60 then
    bbs.println("Good job! Keep learning!")
elseif percent >= 40 then
    bbs.println("Not bad. Room for improvement!")
else
    bbs.println("Keep studying! You'll do better next time!")
end

if score > high_score then
    bbs.println("")
    bbs.println("*** NEW HIGH SCORE! ***")
    bbs.user_data.set("high_score", score)
end
