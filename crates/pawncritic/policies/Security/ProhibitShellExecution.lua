-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

local parsed_options, function_set
local function read_functions(options)
    if parsed_options ~= options then
        parsed_options = options
        function_set = pawn.set(options.functions)
    end
    return function_set
end

return {
    name = "Security::ProhibitShellExecution",
    severity = 4,
    themes = { "core", "security" },
    summary = "the plugin runs a program on the server's machine",
    description = [[
        Extensions such as System2 let a plugin run any program with the rights of the game server:
        reading and changing every file the server may, starting processes, opening connections. A
        plugin that does so needs a careful review. Reported is every call of a function of
        `functions`.
    ]],
    options = {
        functions = {
            default = {
                "System2_Execute", "System2_ExecuteThreaded", "System2_ExecuteFormatted",
                "System2_ExecuteFormattedThreaded",
            },
            description = "Functions that run a program.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            local name, is_method = calls.name(node)
            if not is_method and read_functions(ctx.options)[name] then
                ctx:report(node, ("%s() runs a program on the server's machine"):format(name))
            end
        end,
    },
}
