-- Copyright 2026 davenonymous

local variable_kinds = pawn.set({
    "variable_declaration", "old_variable_declaration", "parameter_declaration", "dynamic_array_declaration",
    "enum_struct_field",
})
-- Only the functions whose name the author chooses: a constructor and a destructor are named after
-- their methodmap, a property accessor is `get` or `set` and an operator overload is its operator,
-- so the functions library's full list of kinds is no fit here.
local function_kinds = pawn.set({ "function_definition", "enum_struct_method", "methodmap_method" })

local function lowered_set(list)
    local set = {}
    for _, item in ipairs(list) do
        set[item:lower()] = true
    end
    return set
end

-- The option lists as sets, built once: the options of a policy's state
-- never change, so the sets are kept for as long as ctx.options is the
-- same table.
local sets_options, sets
local function option_sets(options)
    if sets_options ~= options then
        sets_options = options
        sets = {
            variable_names = lowered_set(options.variable_names),
            short_names = pawn.set(options.short_names),
            function_names = lowered_set(options.function_names),
        }
    end
    return sets
end

local function check_variable(node, ctx)
    local name_node = node:child_by_field("name")
    if not name_node then
        return
    end
    local name = name_node:text()
    local names = option_sets(ctx.options)
    if names.variable_names[name:lower()] then
        ctx:report(name_node, ("`%s` says nothing about what the variable holds"):format(name))
    elseif #name == 1 and not names.short_names[name] then
        ctx:report(name_node, ("the one-letter name `%s` says nothing about what the variable holds"):format(name))
    end
end

local function check_function(node, ctx)
    local name_node = node:child_by_field("name")
    local name = name_node:text()
    if option_sets(ctx.options).function_names[name:lower()] then
        ctx:report(name_node, ("`%s` says nothing about what the function does"):format(name))
    end
end

local handlers = {}
for kind in pairs(variable_kinds) do
    handlers[kind] = check_variable
end
for kind in pairs(function_kinds) do
    handlers[kind] = check_function
end

return {
    name = "Philosophy::ProhibitMeaninglessNames",
    severity = 2,
    themes = { "core", "philosophy", "naming" },
    summary = "the name says nothing about its purpose",
    description = [[
        The Law of Intentional Naming: names carry the meaning, so the code reads like a sentence
        and needs no comment to explain it. `tmp`, `foo`, `stuff` or `check()` make the reader
        work it out. Variables and functions named from `variable_names` and `function_names`,
        ignoring case, are reported, and one-letter variable names other than `short_names`.
    ]],
    options = {
        variable_names = {
            default = { "tmp", "temp", "foo", "bar", "baz", "qux", "stuff", "thing", "things", "dummy", "blah", "asdf", "obj", "var", "val", "data2", "tmp2", "temp2" },
            description = "Variable names to report, ignoring case.",
        },
        function_names = {
            default = { "check", "process", "handle", "doit", "do_it", "dostuff", "do_stuff", "func", "foo", "bar", "test", "temp", "helper" },
            description = "Function names to report, ignoring case.",
        },
        short_names = {
            default = { "i", "j", "k", "x", "y", "z" },
            description = "One-letter variable names that are fine.",
        },
    },
    on = handlers,
}
