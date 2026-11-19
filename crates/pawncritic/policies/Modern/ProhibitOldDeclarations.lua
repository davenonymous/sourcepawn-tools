-- Copyright 2026 davenonymous

local old_variable_kinds = pawn.set({
    "old_variable_declaration_statement",
    "old_global_variable_declaration",
    "old_for_loop_variable_declaration_statement",
})

local function report_old_variable(node, ctx)
    local keyword = node:text():match("^(%a+)")
    if keyword == "decl" then
        ctx:report(node, "`decl` declares a variable without initializing it; declare it with its type, as in `int count;`")
        return
    end
    ctx:report(node)
end

local function report_missing_return_type(node, ctx)
    if node:child_by_field("returnType") then
        return
    end
    local name = node:child_by_field("name")
    ctx:report(name, ("function `%s` has no return type; declare it, as in `void %s(...)`"):format(name:text(), name:text()))
end

local handlers = {
    function_definition = report_missing_return_type,
    function_declaration = report_missing_return_type,
}
for kind in pairs(old_variable_kinds) do
    handlers[kind] = report_old_variable
end

return {
    name = "Modern::ProhibitOldDeclarations",
    severity = 4,
    themes = { "core", "modern" },
    summary = "old-style declaration; declare the variable with its type, as in `int count = 0;`",
    description = [[
        SourcePawn 1.7 introduced typed declarations: `int count = 0;` instead of `new count = 0;`,
        `char name[64];` instead of `decl String:name[64];`, and functions with a return type,
        `void OnPluginStart()` instead of `OnPluginStart()`. Old declarations hide the type in a tag,
        and `decl` leaves the variable uninitialized. With `#pragma newdecls required` the compiler
        rejects them; see Modern::RequireNewDecls.
    ]],
    on = handlers,
}
