-- Copyright 2026 davenonymous

local functions = pawn.library("functions")

local loop_kinds = pawn.set({ "for_statement", "while_statement", "do_while_statement" })

-- The statements of a block, without comments.
local function statements(block)
    local found = {}
    for _, child in ipairs(block:named_children()) do
        if child:kind() ~= "comment" then
            found[#found + 1] = child
        end
    end
    return found
end

return {
    name = "Philosophy::RequireEarlyExit",
    severity = 2,
    themes = { "core", "philosophy" },
    summary = "the rest of the body is wrapped in a condition; return or continue early instead",
    description = [[
        The Law of the Early Exit: handle edge cases at the top, then do the work unindented. A
        function or loop body that ends in an `if` without `else`, around `min_statements` or more
        statements, nests its main work in a condition. Invert the condition and leave early:
        `if (!IsValidClient(client)) { return; }` in a function, `continue;` in a loop.
    ]],
    options = {
        min_statements = { default = 3, description = "The fewest statements in the wrapping `if` that are reported." },
    },
    on = {
        block = function(block, ctx)
            local owner = block:parent()
            if not owner then
                return
            end
            local in_loop = loop_kinds[owner:kind()] and block:field() == "body"
            local in_function = functions.kind_set[owner:kind()] and block:field() == "body"
            if not (in_loop or in_function) then
                return
            end
            local body = statements(block)
            local last = body[#body]
            if not last or last:kind() ~= "condition_statement" or last:child_by_field("falsePath") then
                return
            end
            local true_path = last:child_by_field("truePath")
            if true_path:kind() ~= "block" or #statements(true_path) < ctx.options.min_statements then
                return
            end
            local exit = in_loop and "continue" or "return"
            ctx:report(last, nil, { hint = ("invert the condition and `%s` early, then do the work unindented"):format(exit) })
        end,
    },
}
