-- Copyright 2026 davenonymous

local jump_kinds = pawn.set({ "return_statement", "break_statement", "continue_statement" })

-- The last statement of `node`, looking into blocks and skipping comments.
local function last_statement(node)
    if node:kind() ~= "block" then
        return node
    end
    local children = node:named_children()
    for index = #children, 1, -1 do
        if children[index]:kind() ~= "comment" then
            return last_statement(children[index])
        end
    end
end

return {
    name = "Philosophy::ProhibitElseAfterReturn",
    severity = 2,
    themes = { "core", "philosophy" },
    summary = "`else` after a branch that leaves; drop the `else` and unindent its body",
    description = [[
        The Law of the Early Exit: when the `if` branch ends in `return`, `break` or `continue`,
        the code after it only runs when the condition is false. An `else` around that code adds a
        level of indentation and nothing else.
    ]],
    on = {
        condition_statement = function(node, ctx)
            if not node:child_by_field("falsePath") then
                return
            end
            local last = last_statement(node:child_by_field("truePath"))
            if not last or not jump_kinds[last:kind()] then
                return
            end
            for _, child in ipairs(node:children()) do
                if child:kind() == "else" then
                    local jump = last:text():match("^%a+")
                    ctx:report(child, ("`else` after `%s`; drop it and unindent its body"):format(jump))
                    return
                end
            end
        end,
    },
}
