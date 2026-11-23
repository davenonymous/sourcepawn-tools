-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

-- The ConVar patterns as a list of { prefix, exact } pairs, built once per
-- options table: `name*` matches every name that starts with `name`.
local parsed_options, patterns
local function read_patterns(options)
    if parsed_options ~= options then
        parsed_options = options
        patterns = {}
        for _, entry in ipairs(options.convars) do
            local lowered = pawn.trim(entry):lower()
            local prefix = lowered:match("^(.*)%*$")
            patterns[#patterns + 1] = { text = prefix or lowered, is_prefix = prefix ~= nil }
        end
    end
    return patterns
end

-- The first ConVar of the list that a word of the text names, or nil.
local function named_convar(text, list)
    for word in text:lower():gmatch("[%w_]+") do
        for _, pattern in ipairs(list) do
            local matches = pattern.is_prefix and pawn.starts_with(word, pattern.text) or word == pattern.text
            if matches then
                return word
            end
        end
    end
    return nil
end

-- Whether a node is part of a `...` concatenation, which is checked whole.
local function is_concatenated(node)
    local parent = node:parent()
    return parent ~= nil and parent:kind() == "binary_expression" and calls.string_value(parent) ~= nil
end

-- Reports a string, or a concatenation of strings, that names a ConVar.
local function report_convar(node, ctx)
    local value = calls.string_value(node)
    local convar = value and not is_concatenated(node) and named_convar(value, read_patterns(ctx.options))
    if convar then
        ctx:report(node, ("the code names the ConVar `%s`"):format(convar), {
            hint = "a plugin never needs the RCON password; remove this",
        })
    end
end

return {
    name = "Malicious::ProhibitPasswordConVars",
    severity = 5,
    themes = { "core", "malicious", "security" },
    summary = "the code names the RCON password ConVar",
    description = [[
        A plugin has no business with the RCON password: whoever knows it controls the server. Code
        that reads it with `FindConVar("rcon_password")`, sets it with `ServerCommand()` or names it
        in any other string is the classic way of stealing it or locking the owner out. Every string
        literal that names one of the ConVars in `convars` as a word is reported.
    ]],
    options = {
        convars = {
            default = { "rcon_password", "sv_rcon_*" },
            description = "ConVar names, ignoring case; a trailing * matches any rest.",
        },
    },
    on = {
        string_literal = function(node, ctx)
            report_convar(node, ctx)
        end,
        binary_expression = function(node, ctx)
            report_convar(node, ctx)
        end,
    },
}
