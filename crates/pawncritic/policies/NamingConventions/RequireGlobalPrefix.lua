-- Copyright 2026 davenonymous

local function is_constant(declaration, name)
    local storage = declaration:child_by_field("storage_class")
    return storage ~= nil or name:match("^[%u%d_]+$") ~= nil
end

return {
    name = "NamingConventions::RequireGlobalPrefix",
    severity = 1,
    themes = { "core", "naming" },
    summary = "a global variable without the global prefix",
    description = [[
        SourceMod plugins mark global variables with a prefix, `g_` by convention, so a reader sees
        at once that a function touches state outside itself. Constants (`const` or all upper case
        names) are not reported. The option `prefix` sets the prefix.
    ]],
    options = {
        prefix = { default = "g_", description = "The prefix global variable names start with." },
    },
    on = {
        global_variable_declaration = function(node, ctx)
            for _, variable in ipairs(node:named_children()) do
                local name = variable:kind() == "variable_declaration" and variable:child_by_field("name")
                if name and not is_constant(node, name:text()) and not pawn.starts_with(name:text(), ctx.options.prefix) then
                    ctx:report(name, ("the global `%s` lacks the prefix `%s`"):format(name:text(), ctx.options.prefix))
                end
            end
        end,
        old_global_variable_declaration = function(node, ctx)
            for _, variable in ipairs(node:named_children()) do
                local name = variable:kind() == "old_variable_declaration" and variable:child_by_field("name")
                if name and not is_constant(node, name:text()) and not pawn.starts_with(name:text(), ctx.options.prefix) then
                    ctx:report(name, ("the global `%s` lacks the prefix `%s`"):format(name:text(), ctx.options.prefix))
                end
            end
        end,
    },
}
