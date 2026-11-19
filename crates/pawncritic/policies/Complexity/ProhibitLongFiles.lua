-- Copyright 2026 davenonymous

local lines = pawn.library("lines")

return {
    name = "Complexity::ProhibitLongFiles",
    severity = 3,
    themes = { "core", "complexity", "philosophy" },
    summary = "the file has too many lines",
    description = [[
        A long file mixes many concerns, and finding the part one is looking for takes searching
        instead of reading. Split it into include files by topic, such as
        `scripting/include/<plugin>/menus.inc`, and keep the plugin's `.sp` file for what ties
        them together. A `.sp` or `.inc` file with more than `max_lines` code lines (default 500)
        is reported on its first line; blank lines and lines that hold only comments do not count.
    ]],
    options = {
        max_lines = { default = 500, description = "The most code lines a file may have." },
    },
    check = function(file, ctx)
        local max_lines = ctx.options.max_lines
        local _, count = lines.code_lines(file)
        if count <= max_lines then
            return
        end
        ctx:report({ line = 1 }, ("the file has %d code lines, more than %d"):format(count, max_lines), {
            hint = "split it into include files by topic",
        })
    end,
}
