-- Copyright 2026 davenonymous

local owners = {
    condition_statement = "condition",
    for_statement = "loop",
    while_statement = "loop",
    do_while_statement = "loop",
    switch_case = "case",
}

return {
    name = "Philosophy::ProhibitEmptyBlocks",
    severity = 3,
    themes = { "core", "philosophy" },
    summary = "an empty block silently does nothing",
    description = [[
        The Law of Fail Fast, Fail Loud: a branch or loop that does nothing hides a case nobody
        decided about. Remove the branch, handle the case, or say in a comment inside the braces
        why nothing has to happen there; a block with a comment is not reported.
    ]],
    on = {
        block = function(block, ctx)
            local owner = block:parent()
            local what = owner and owners[owner:kind()]
            if not what then
                return
            end
            if #block:named_children() == 0 then
                ctx:report(block, ("the %s has an empty body"):format(what), {
                    hint = "handle the case, remove it, or explain in a comment why nothing happens",
                })
            end
        end,
    },
}
