-- Copyright 2026 davenonymous

return {
    name = "Modern::ProhibitCloseHandle",
    severity = 4,
    themes = { "core", "modern" },
    summary = "`CloseHandle()` is superseded by `delete`",
    description = [[
        `delete handle;` closes a handle and sets the variable to `null`, so the variable can never
        point to a closed handle. `CloseHandle(handle)` leaves the stale value behind, and a second
        close or a later use fails at run time.
    ]],
    query = [[
        (call_expression
            function: (identifier) @function (#eq? @function "CloseHandle")
            arguments: (call_arguments) @arguments) @call
    ]],
    on_match = function(captures, ctx)
        local argument = captures.arguments:text():sub(2, -2)
        ctx:report(captures.call, nil, { hint = ("delete %s;"):format(argument) })
    end,
}
