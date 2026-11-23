-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local functions = pawn.library("functions")
local taint = pawn.library("taint")

local parsed_options, format_specs, taint_specs
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        format_specs = calls.parse_positions(options.format_functions, "format_functions")
        taint_specs = {
            sources = calls.parse_positions(options.sources, "sources"),
            forwards = calls.parse_positions(options.forwards, "forwards"),
            propagators = calls.parse_positions(options.propagators, "propagators"),
            sanitizers = calls.parse_positions(options.sanitizers, "sanitizers"),
        }
    end
    return format_specs, taint_specs
end

local sql_keywords = { "select", "insert", "update", "delete", "replace" }

-- Whether a template is SQL: one of its words is an SQL keyword.
local function is_sql(template)
    for word in template:lower():gmatch("%a+") do
        for _, keyword in ipairs(sql_keywords) do
            if word == keyword then
                return true
            end
        end
    end
    return false
end

-- The conversions of a SourceMod format, in order, as their letters: `%T`
-- takes two arguments and counts twice, `%%` none.
local function conversions(template)
    local letters = {}
    for flags, letter in template:gmatch("%%([-+ 0#%d.]*)(.)") do
        if letter ~= "%" or flags ~= "" then
            letters[#letters + 1] = letter
            if letter == "T" then
                letters[#letters + 1] = "T"
            end
        end
    end
    return letters
end

return {
    name = "Security::ProhibitSqlInjection",
    severity = 5,
    themes = { "bugs", "core", "security" },
    summary = "client input goes into an SQL query unescaped",
    description = [[
        A player's name or chat message may hold quotes and SQL. Formatted into a query with `%s`, it
        becomes part of the query and can read or destroy the database. Format queries with
        `Database.Format()` or `SQL_FormatQuery()`, whose `%s` escapes, or escape the text with
        `SQL_EscapeString()` or `Database.Escape()` first. Reported is a variable holding client
        input that a function of `format_functions` formats with `%s` into a template with an SQL
        keyword (SELECT, INSERT, UPDATE, DELETE, REPLACE). Client input is tracked within one
        function as described for `Security::ProhibitCommandInjection`; a buffer that one of
        `sanitizers` writes no longer holds it.
    ]],
    options = {
        format_functions = {
            default = { "Format:3", "FormatEx:3" },
            description = "Functions (not methods) that format text, as Name:N with N the template's position.",
        },
        sources = {
            default = taint.default_sources,
            description = "Calls that fill a buffer with client input, as Name:N with N the buffer's position.",
        },
        forwards = {
            default = taint.default_forwards,
            description = "Forwards whose parameters hold client input, as Name:N with N the position.",
        },
        propagators = {
            default = taint.default_propagators,
            description = "Calls that write their other arguments into a buffer, as Name:N with N its position.",
        },
        sanitizers = {
            default = { "SQL_EscapeString:3", "Escape:2" },
            description = "Calls that write escaped text into a buffer, as Name:N with N its position.",
        },
    },
    check = function(file, ctx)
        local formatters, specs = read_options(ctx.options)
        for _, function_node in ipairs(file:find(functions.kinds)) do
            local is_tainted = taint.analyze(function_node, specs)
            for _, call in ipairs(function_node:find("call_expression")) do
                local name, is_method = calls.name(call)
                local position = not is_method and formatters[name] and formatters[name][1]
                local list = position and calls.arguments(call) or {}
                local template = list[position] and calls.string_value(list[position])
                if template and is_sql(template) then
                    for index, letter in ipairs(conversions(template)) do
                        local argument = list[position + index]
                        if letter == "s" and argument and is_tainted(argument) then
                            ctx:report(argument, ("`%s` holds client input and goes into a query unescaped"):format(
                                argument:text()), { hint = "format the query with Database.Format()" })
                        end
                    end
                end
            end
        end
    end,
}
