-- Copyright 2026 davenonymous

-- Containers whose opening brace is a child of their own.
local containers = pawn.set({
    "enum_struct", "methodmap", "methodmap_property", "struct", "switch_statement", "typeset", "funcenum",
})

-- The node before `node` among its siblings, skipping comments.
local function previous_code_sibling(node)
    local sibling = node:prev_sibling()
    while sibling and sibling:kind() == "comment" do
        sibling = sibling:prev_sibling()
    end
    return sibling
end

local function check_brace(brace, before, ctx)
    if not before then
        return
    end
    if before:end_line() ~= brace:line() then
        ctx:report(brace, "the opening brace is not on the line of its statement",
            { hint = ("move `{` up to line %d"):format(before:end_line()) })
    end
end

local function check_block_like(node, ctx)
    local parent = node:parent()
    if not parent or parent:kind() == "block" or parent:kind() == "source_file" then
        return
    end
    check_brace(node:child(1), previous_code_sibling(node), ctx)
end

local function check_container(node, ctx)
    for _, child in ipairs(node:children()) do
        if child:kind() == "{" then
            check_brace(child, previous_code_sibling(child), ctx)
            return
        end
    end
end

local handlers = { block = check_block_like, enum_entries = check_block_like }
for kind in pairs(containers) do
    handlers[kind] = check_container
end

return {
    name = "CodeLayout::RequireSameLineBraces",
    severity = 3,
    themes = { "core", "layout", "tidy" },
    summary = "the opening brace is not on the line of its statement",
    description = [[
        An opening brace stands at the end of the line that starts its function, condition, loop,
        enum, methodmap or other block, not on a line of its own: `if (ready) {`. A nested block
        that is a statement of its own is not checked.
    ]],
    on = handlers,
}
