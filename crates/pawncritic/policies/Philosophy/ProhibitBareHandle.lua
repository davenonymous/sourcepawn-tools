-- Copyright 2026 davenonymous

-- Functions whose result has a methodmap, with that methodmap. A `Handle`
-- variable that takes one of these results should be declared with it.
local result_types = {
    CreateArray = "ArrayList",
    CloneArray = "ArrayList",
    ReadMapList = "ArrayList",
    CreateStack = "ArrayStack",
    CreateTrie = "StringMap",
    CreateTrieSnapshot = "StringMapSnapshot",
    CreateDataPack = "DataPack",
    CreateKeyValues = "KeyValues",
    CreateMenu = "Menu",
    CreatePanel = "Panel",
    CreateConVar = "ConVar",
    FindConVar = "ConVar",
    OpenFile = "File",
    OpenDirectory = "DirectoryListing",
    SQL_Connect = "Database",
    SQL_DefConnect = "Database",
    SQL_ConnectCustom = "Database",
    SQL_Query = "DBResultSet",
    SQL_PrepareQuery = "DBStatement",
    CreateGlobalForward = "GlobalForward",
    CreateForward = "PrivateForward",
    CreateEvent = "Event",
    CreateProfiler = "Profiler",
    CompileRegex = "Regex",
    RegClientCookie = "Cookie",
    FindClientCookie = "Cookie",
    CreateTopMenu = "TopMenu",
    GetAdminTopMenu = "TopMenu",
    LoadGameConfigFile = "GameData",
}

local function is_handle_type(type_node)
    local text = type_node:text()
    return text == "Handle" or text:match("^Handle%s*:$") ~= nil
end

-- The methodmap of the value `expression` produces, if it is a known call
-- or a `new` of a methodmap.
local function methodmap_of(expression, extra)
    if not expression then
        return nil
    end
    if expression:kind() == "new_expression" then
        return expression:child_by_field("class"):text(), "new " .. expression:child_by_field("class"):text() .. "()"
    end
    if expression:kind() ~= "call_expression" then
        return nil
    end
    local callee = expression:child_by_field("function")
    if callee:kind() ~= "identifier" then
        return nil
    end
    local methodmap = extra[callee:text()] or result_types[callee:text()]
    if methodmap then
        return methodmap, callee:text() .. "()"
    end
end

local function extra_types(list)
    local extra = {}
    for _, entry in ipairs(list) do
        local name, methodmap = entry:match("^%s*([%w_]+)%s*=%s*([%w_]+)%s*$")
        if name then
            extra[name] = methodmap
        end
    end
    return extra
end

local functions = pawn.library("functions")

-- The initial value of a variable declaration. In old-style declarations
-- the field also holds the `=`, so the named child is the value.
local function initial_value(variable)
    for _, child in ipairs(variable:children_by_field("initialValue")) do
        if child:is_named() then
            return child
        end
    end
end

-- The variables declared as `Handle` in a declaration statement, by name.
local function handle_variables(declaration)
    local found = {}
    local shared_type = declaration:child_by_field("type")
    for _, variable in ipairs(declaration:named_children()) do
        local kind = variable:kind()
        if kind == "variable_declaration" or kind == "old_variable_declaration" then
            local type_node = variable:child_by_field("type") or shared_type
            local name = variable:child_by_field("name")
            if type_node and name and is_handle_type(type_node) then
                found[#found + 1] = { name = name, initial = initial_value(variable) }
            end
        end
    end
    return found
end

-- Where `name` is assigned inside `scope`, the methodmap of the first
-- assigned value that has one.
local function assigned_methodmap(scope, name, extra)
    for _, assignment in ipairs(scope:find("assignment_expression")) do
        local left = assignment:child_by_field("left")
        if left:kind() == "identifier" and left:text() == name then
            local methodmap, source = methodmap_of(assignment:child_by_field("right"), extra)
            if methodmap then
                return methodmap, source
            end
        end
    end
end

local function report(ctx, variable, methodmap, source)
    local name = variable.name:text()
    ctx:report(variable.name, ("`%s` stores the result of %s; declare it as `%s %s`"):format(name, source, methodmap, name))
end

return {
    name = "Philosophy::ProhibitBareHandle",
    severity = 3,
    themes = { "core", "philosophy", "modern" },
    summary = "a `Handle` variable holds a handle that has a methodmap; declare it with the methodmap",
    description = [[
        Make illegal states unrepresentable: a variable of type `Handle` can hold any handle, and
        the compiler cannot tell a menu from a timer. Declared as `ArrayList`, `StringMap`,
        `ConVar`, `Menu`, `KeyValues` and so on, it only takes what it should, and its members
        replace the old functions. This policy reports a `Handle` variable that is given the
        result of a function whose result has a methodmap, such as `CreateArray()` or
        `FindConVar()`, or a `new ArrayList(...)`. Handles without a methodmap, such as timers, HUD
        synchronizers or SDK calls, stay `Handle` and are not reported. The option `extra` adds
        functions as `"Function=Methodmap"`, for the natives of extensions.
    ]],
    options = {
        extra = { default = {}, description = "More functions whose result has a methodmap, as \"Function=Methodmap\"." },
    },
    check = function(file, ctx)
        local extra = extra_types(ctx.options.extra)
        local root = file:root()
        for _, declaration in ipairs(file:find("global_variable_declaration", "old_global_variable_declaration")) do
            for _, variable in ipairs(handle_variables(declaration)) do
                local methodmap, source = methodmap_of(variable.initial, extra)
                if not methodmap then
                    methodmap, source = assigned_methodmap(root, variable.name:text(), extra)
                end
                if methodmap then
                    report(ctx, variable, methodmap, source)
                end
            end
        end
        for _, declaration in ipairs(file:find("variable_declaration_statement", "old_variable_declaration_statement")) do
            local scope = declaration:ancestor(functions.kinds) or root
            for _, variable in ipairs(handle_variables(declaration)) do
                local methodmap, source = methodmap_of(variable.initial, extra)
                if not methodmap then
                    methodmap, source = assigned_methodmap(scope, variable.name:text(), extra)
                end
                if methodmap then
                    report(ctx, variable, methodmap, source)
                end
            end
        end
    end,
}
