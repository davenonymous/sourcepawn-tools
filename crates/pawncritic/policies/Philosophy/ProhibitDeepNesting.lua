-- Copyright 2026 davenonymous

local nesting_kinds = pawn.set({
    "condition_statement", "for_statement", "while_statement", "do_while_statement", "switch_statement",
})

local functions = pawn.library("functions")

-- An `else if` continues the chain of its `if` and adds no level.
local function is_else_if(node)
    return node:kind() == "condition_statement" and node:field() == "falsePath"
end

-- How many conditions and loops `node` is nested in, inside its function.
local function depth_of(node)
    local depth = 0
    local current = node
    for ancestor in node:ancestors() do
        if functions.kind_set[ancestor:kind()] then
            break
        end
        if nesting_kinds[ancestor:kind()] and not is_else_if(current) then
            depth = depth + 1
        end
        current = ancestor
    end
    return depth
end

local function check(node, ctx)
    if is_else_if(node) then
        return
    end
    local depth = depth_of(node) + 1
    if depth == ctx.options.max_depth + 1 then
        ctx:report(node, ("nested %d levels deep, more than %d"):format(depth, ctx.options.max_depth), {
            hint = "return early, continue early or move the inner part into a function of its own",
        })
    end
end

local handlers = {}
for kind in pairs(nesting_kinds) do
    handlers[kind] = check
end

return {
    name = "Philosophy::ProhibitDeepNesting",
    severity = 3,
    themes = { "core", "philosophy" },
    summary = "conditions and loops are nested too deeply",
    description = [[
        The Law of the Early Exit: indentation is the enemy of simplicity, and deep nesting hides
        bugs. Handle the edge cases at the top with guard clauses (`if (!IsClientInGame(client)) {
        return; }`), skip loop iterations with `continue`, and give an inner part a function of its
        own. A condition or loop nested deeper than `max_depth` levels inside its function is
        reported, once per nest; an `else if` adds no level.
    ]],
    options = {
        max_depth = { default = 3, description = "The deepest nesting of conditions and loops allowed." },
    },
    on = handlers,
}
