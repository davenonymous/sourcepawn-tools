-- Copyright 2026 davenonymous

return {
    name = "CodeLayout::ProhibitTrailingWhitespace",
    severity = 2,
    themes = { "core", "layout", "tidy" },
    summary = "the line ends in whitespace",
    description = [[
        Whitespace at the end of a line is invisible, changes with every editor that strips it and
        clutters diffs. pawntidy removes it.
    ]],
    check = function(file, ctx)
        for number, text in file:lines() do
            -- Scanned back from the end: the pattern `[ \t]+$` would retry from every
            -- blank of a run in the middle of the line, quadratic inside one C call.
            local last_code = #text
            while last_code > 0 and (text:byte(last_code) == 32 or text:byte(last_code) == 9) do
                last_code = last_code - 1
            end
            if last_code < #text then
                ctx:report({ line = number, column = utf8.len(text:sub(1, last_code)) + 1, end_column = utf8.len(text) + 1 })
            end
        end
    end,
}
