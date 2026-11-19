-- Copyright 2026 davenonymous

local functions = pawn.library("functions")

local function returns_bool(function_node)
    local return_type = function_node:child_by_field("returnType")
    return return_type ~= nil and (return_type:text() == "bool" or return_type:text():match("^bool%s*:$") ~= nil)
end

local function has_predicate_prefix(name, prefixes)
    local bare = name:gsub("^_+", "")
    local lowered = bare:lower()
    for _, prefix in ipairs(prefixes) do
        local candidate = prefix:lower()
        if lowered:sub(1, #candidate) == candidate then
            local after = bare:sub(#candidate + 1, #candidate + 1)
            if after == "" or after == "_" or after:match("%u") or after:match("%d") then
                return true
            end
        end
    end
    return false
end

return {
    name = "Philosophy::RequireBooleanPredicateNames",
    severity = 1,
    themes = { "core", "philosophy", "naming" },
    summary = "a function returning bool should be named as a question",
    description = [[
        The Law of Intentional Naming: `IsUserEligible(client)` guarantees by its name what the
        boolean means, `CheckUser(client)` does not. A non-public function returning `bool` should
        start with one of `prefixes`, ignoring case and leading underscores, followed by an upper
        case letter, a digit, an underscore or nothing. Public functions are forwards and
        callbacks with names chosen by the API.
    ]],
    options = {
        prefixes = {
            default = { "is", "has", "have", "can", "could", "should", "was", "were", "will", "are", "does", "did", "needs", "contains", "exists", "allows", "supports", "matches", "equals", "try" },
            description = "The words a predicate name may start with.",
        },
    },
    on = {
        function_definition = function(node, ctx)
            if not returns_bool(node) or functions.is_public(node) then
                return
            end
            local name = node:child_by_field("name")
            if not has_predicate_prefix(name:text(), ctx.options.prefixes) then
                ctx:report(name, ("`%s` returns bool; name it as a question, like `Is%s`"):format(name:text(), (name:text():gsub("^%l", string.upper))))
            end
        end,
    },
}
