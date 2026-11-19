-- Copyright 2026 davenonymous

return {
    name = "CodeLayout::ProhibitLongLines",
    severity = 2,
    themes = { "core", "layout", "tidy" },
    summary = "the line is longer than the limit",
    description = [[
        Long lines are hard to read side by side and in reviews. Lines may be at most `max_length`
        columns wide (default 120), with a tab counting up to the next multiple of `tab_width`
        (default 4). pawntidy wraps long lines where it can.
    ]],
    options = {
        max_length = { default = 120, description = "The widest a line may be, in columns." },
        tab_width = { default = 4, description = "How many columns a tab stop is wide." },
    },
    check = function(file, ctx)
        local max_length = ctx.options.max_length
        for number, text in file:lines() do
            local width = pawn.display_width(text, ctx.options.tab_width)
            if width > max_length then
                ctx:report({ line = number }, ("the line is %d columns wide, the limit is %d"):format(width, max_length))
            end
        end
    end,
}
