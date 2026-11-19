-- Copyright 2026 davenonymous

local functions = pawn.library("functions")
local lines = pawn.library("lines")

local function count_code_lines(code_lines, first, last)
    local count = 0
    for line = first, last do
        if code_lines[line] then
            count = count + 1
        end
    end
    return count
end

-- Whether a body holds nothing but `MarkNativeAsOptional()` calls, such as the
-- `__pl_<name>_SetNTVOptional` of a library's include: a flat list with no logic to split.
local function is_optional_native_list(body)
    local calls = 0
    for _, child in ipairs(body:named_children()) do
        if child:kind() ~= "comment" then
            local parts = child:kind() == "expression_statement" and child:named_children() or {}
            local call = #parts == 1 and parts[1]:kind() == "call_expression" and parts[1]
            local callee = call and call:child_by_field("function")
            if not callee or callee:text() ~= "MarkNativeAsOptional" then
                return false
            end
            calls = calls + 1
        end
    end
    return calls > 0
end

-- The name a reader knows the function by; a property's accessors go by the property.
local function name_of(node)
    local name = node:child_by_field("name")
    if name then
        return name:text()
    end
    local property = node:ancestor("methodmap_property")
    if property and property:child_by_field("name") then
        return property:child_by_field("name"):text()
    end
    return node:kind()
end

return {
    name = "Complexity::ProhibitLongFunctions",
    severity = 3,
    themes = { "core", "complexity", "philosophy" },
    summary = "the function has too many lines",
    description = [[
        A long function does several things at once: its reader has to keep all of them in mind,
        and a change to one risks the others. Split it into smaller functions that each do one
        thing and are named for it. A function, method, constructor, destructor or property
        accessor with more than `max_lines` code lines (default 50) is reported; blank lines and
        lines that hold only comments do not count. A function whose body is nothing but
        `MarkNativeAsOptional()` calls, such as the `__pl_<name>_SetNTVOptional` of a library's
        include, is a list rather than logic and is not reported.
    ]],
    options = {
        max_lines = { default = 50, description = "The most code lines a function may have." },
    },
    check = function(file, ctx)
        local max_lines = ctx.options.max_lines
        local code_lines = lines.code_lines(file)
        for _, node in ipairs(file:find(functions.kinds)) do
            local body = node:child_by_field("body")
            if body and not is_optional_native_list(body) then
                local count = count_code_lines(code_lines, node:line(), node:end_line())
                if count > max_lines then
                    ctx:report(node, ("`%s` has %d code lines, more than %d"):format(name_of(node), count, max_lines), {
                        hint = "split it into smaller functions that each do one thing",
                    })
                end
            end
        end
    end,
}
