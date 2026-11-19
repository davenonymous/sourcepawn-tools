-- Copyright 2026 davenonymous

local functions = pawn.library("functions")

local global_kinds = { "global_variable_declaration", "old_global_variable_declaration" }
local local_kinds = { "variable_declaration", "old_variable_declaration", "parameter_declaration", "dynamic_array_declaration" }

-- The names of the variables declared by global declarations.
local function global_names(file)
    local names = {}
    for _, declaration in ipairs(file:find(global_kinds)) do
        if declaration:parent():kind() == "source_file" then
            for _, variable in ipairs(declaration:named_children()) do
                local name = variable:child_by_field("name")
                if name then
                    names[name:text()] = true
                end
            end
        end
    end
    return names
end

-- The variable an assignment target writes to: `g_x` or an element `g_list[i]`. A write to a
-- field (`g_data.field`) is not followed: without types, an enum struct field cannot be told
-- apart from a methodmap property.
local function written_name(target)
    while target and (target:kind() == "array_indexed_access" or target:kind() == "packed_array_indexed_access") do
        target = target:child_by_field("array")
    end
    if target and target:kind() == "identifier" then
        return target:text()
    end
end

local function returns_void(function_node)
    local return_type = function_node:child_by_field("returnType")
    return return_type == nil or return_type:text() == "void" or return_type:text() == "void:"
end

return {
    name = "Philosophy::ProhibitHiddenGlobalMutation",
    severity = 2,
    themes = { "core", "philosophy" },
    summary = "a void function changes global state as a side effect",
    description = [[
        The Law of Atomic Predictability: a function should not surprise its caller. A non-public
        function that returns nothing but writes to global variables changes state the caller
        cannot see at the call. Return the new value and let the caller store it, or pass what
        the function changes as a reference parameter. Public functions are callbacks and forwards
        whose job is to change state, and are not reported.
    ]],
    check = function(file, ctx)
        local globals = global_names(file)
        if next(globals) == nil then
            return
        end
        for _, function_node in ipairs(file:find("function_definition")) do
            if returns_void(function_node) and not functions.is_public(function_node) then
                local locals = {}
                for _, declaration in ipairs(function_node:find(local_kinds)) do
                    local name = declaration:child_by_field("name")
                    if name then
                        locals[name:text()] = true
                    end
                end
                local changed, order = {}, {}
                for _, write in ipairs(function_node:find("assignment_expression", "update_expression")) do
                    local target = write:child_by_field("left") or write:child_by_field("argument")
                    local name = written_name(target)
                    if name and globals[name] and not locals[name] and not changed[name] then
                        changed[name] = true
                        order[#order + 1] = name
                    end
                end
                if #order > 0 then
                    local name = function_node:child_by_field("name")
                    ctx:report(name, ("void function `%s` changes the global%s %s"):format(
                        name:text(), #order > 1 and "s" or "", table.concat(order, ", ")), {
                        hint = "return the new value, or take what it changes as a parameter",
                    })
                end
            end
        end
    end,
}
