-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local functions = pawn.library("functions")

local parsed_options, panel_functions, allowed
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        panel_functions = pawn.set(options.panel_functions)
        allowed = pawn.set(options.allowed_commands)
    end
    return panel_functions, allowed
end

-- Whether the function shows a VGUI panel: it calls a panel function or names
-- the VGUIMenu user message.
local function shows_panel(function_node, panel_set)
    for _, call in ipairs(function_node:find(calls.call_kinds)) do
        if panel_set[calls.name(call)] then
            return true
        end
    end
    for _, literal in ipairs(function_node:find("string_literal")) do
        if calls.string_value(literal) == "VGUIMenu" then
            return true
        end
    end
    return false
end

-- The value that belongs to a `cmd` key: the next argument of its own call,
-- or else the last argument of the next call in the function (the next
-- BfWriteString, PbSetString or SetString).
local function value_of(key, function_calls)
    local next_argument = key:next_named_sibling()
    while next_argument and next_argument:kind() == "comment" do
        next_argument = next_argument:next_named_sibling()
    end
    if next_argument then
        return next_argument
    end
    local key_call = key:parent():parent()
    for index, call in ipairs(function_calls) do
        if call == key_call then
            local following = function_calls[index + 1]
            local list = following and calls.arguments(following) or {}
            return list[#list]
        end
    end
    return nil
end

-- Whether a value is harmless: an integer, a name in capitals (a macro or
-- enum constant) or a literal in the allow-list.
local function is_allowed(value, allowed_set)
    if value == nil then
        return true
    end
    if value:kind() == "int_literal" then
        return true
    end
    if value:kind() == "identifier" and value:text():find("^[A-Z][A-Z0-9_]*$") then
        return true
    end
    local text = calls.string_value(value)
    return text ~= nil and (allowed_set[text] or text:find("^%-?%d+$") ~= nil)
end

return {
    name = "Malicious::ProhibitVguiCommandBypass",
    severity = 5,
    themes = { "core", "malicious", "security" },
    summary = "a VGUI panel with a `cmd` key runs a command on the player's game",
    description = [[
        The `cmd` key of a VGUI panel, such as the MOTD, is a command the player's game runs when the
        panel closes. It was used to run commands that `ClientCommand()` may not run, such as binds
        or `connect`. Reported is a string literal `cmd` used as an argument in a function that shows
        a panel (calls one of `panel_functions` or names the `VGUIMenu` user message), unless its
        value is an integer, a name in capitals (a constant) or one of `allowed_commands`. The value
        is the next argument of the same call, or else the last argument of the next call in the
        function.
    ]],
    options = {
        panel_functions = {
            default = { "ShowVGUIPanel" },
            description = "Functions that show a VGUI panel.",
        },
        allowed_commands = {
            default = { "closed_htmlpage" },
            description = "Harmless values of the cmd key.",
        },
    },
    check = function(file, ctx)
        local panel_set, allowed_set = read_options(ctx.options)
        for _, function_node in ipairs(file:find(functions.kinds)) do
            if shows_panel(function_node, panel_set) then
                local function_calls = function_node:find(calls.call_kinds)
                for _, literal in ipairs(function_node:find("string_literal")) do
                    local is_key = calls.string_value(literal) == "cmd" and literal:parent():kind() == "call_arguments"
                    if is_key and not is_allowed(value_of(literal, function_calls), allowed_set) then
                        ctx:report(literal, nil, { hint = "leave the cmd key out or use closed_htmlpage" })
                    end
                end
            end
        end
    end,
}
