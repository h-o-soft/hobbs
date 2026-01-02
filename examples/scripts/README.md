# HOBBS Sample Scripts

This directory contains sample Lua scripts for the HOBBS BBS script plugin system.

## Installation

Copy these scripts to your HOBBS scripts directory:

```bash
cp examples/scripts/*.lua data/files/scripts/
```

Then use the "Re-sync" option in the Scripts menu (as SubOp or SysOp) to register them.

## Interactive Input Support

**Note:** Interactive input (`bbs.input()`, `bbs.input_number()`, `bbs.input_yn()`) is not yet fully supported. Scripts that require user input will display an error message. The following scripts work without interactive input:

- `hello.lua` - Works (no input required)
- `fortune.lua` - Works (no input required)
- `omikuji.lua` - Works (auto-draws fortune)

Scripts requiring interactive input (will show error):
- `janken.lua`, `number_guess.lua`, `dice.lua`, `quiz.lua`

## Available Scripts

### hello.lua - Hello World
Simple greeting script that demonstrates the BBS API. Shows user info, terminal info, and current date/time.

### fortune.lua - Daily Fortune
Get a random fortune message with:
- Inspirational message
- Lucky numbers
- Lucky color

### omikuji.lua - Fortune Drawing
Traditional Japanese fortune drawing (omikuji). Features:
- Can only draw once per day
- Weighted random fortunes (rare to get "Great Blessing" or "Bad Luck")
- Auto-draws without confirmation

### janken.lua - Rock-Paper-Scissors (requires input)
Classic rock-paper-scissors game against the computer. Tracks win/loss/draw statistics per user.

### number_guess.lua - Number Guessing Game (requires input)
Guess a number between 1 and 100. Features:
- 10 attempts maximum
- High score tracking (fewest attempts)

### dice.lua - Dice Roller (requires input)
Roll various dice:
- D6 (6-sided)
- D20 (20-sided)
- D100 (percentile)
- Custom dice (2-1000 sides)

### quiz.lua - Quiz Game (requires input)
Test your knowledge with trivia questions:
- 5 random questions per game
- Multiple choice answers
- High score tracking

## Script Format

Scripts use metadata headers to define properties:

```lua
-- @name Script Name
-- @description Brief description
-- @author Author Name
-- @min_role 0
```

### min_role Values
- 0 = Guest (anyone)
- 1 = Member (registered users)
- 2 = SubOp
- 3 = SysOp

## BBS API

Scripts have access to the `bbs` global table with functions:

### Output
- `bbs.print(text)` - Output text (no newline)
- `bbs.println(text)` - Output text (with newline)
- `bbs.clear()` - Clear screen (ANSI terminals only)

### Input
- `bbs.input(prompt)` - Get text input
- `bbs.input_number(prompt)` - Get numeric input
- `bbs.input_yn(prompt)` - Get Y/N input (returns boolean)
- `bbs.pause()` - Wait for Enter key

### User Info
- `bbs.get_user()` - Returns table with: id, username, nickname, role
- `bbs.is_guest()` - Check if user is a guest
- `bbs.is_sysop()` - Check if user is SysOp

### Utilities
- `bbs.random(min, max)` - Random integer
- `bbs.sleep(seconds)` - Wait (max 5 seconds)
- `bbs.get_time()` - Current time (HH:MM:SS)
- `bbs.get_date()` - Current date (YYYY-MM-DD)

### Data Persistence
- `bbs.data.get(key)` - Get script-wide data
- `bbs.data.set(key, value)` - Set script-wide data
- `bbs.user_data.get(key)` - Get user-specific data
- `bbs.user_data.set(key, value)` - Set user-specific data

### Terminal Info
- `bbs.terminal.width` - Terminal width
- `bbs.terminal.height` - Terminal height
- `bbs.terminal.has_ansi` - ANSI support flag
