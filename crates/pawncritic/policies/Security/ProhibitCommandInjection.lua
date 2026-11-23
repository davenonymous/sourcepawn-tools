-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local functions = pawn.library("functions")
local taint = pawn.library("taint")

local parsed_options, sinks, specs
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        sinks = calls.parse_positions(options.functions, "functions")
        specs = {
            sources = calls.parse_positions(options.sources, "sources"),
            forwards = calls.parse_positions(options.forwards, "forwards"),
            propagators = calls.parse_positions(options.propagators, "propagators"),
            sanitizers = {},
        }
    end
    return sinks, specs
end

return {
    name = "Security::ProhibitCommandInjection",
    severity = 5,
    themes = { "core", "security" },
    summary = "client input goes into a command",
    description = [[
        A player's name, chat message or command argument may hold `;` and further commands. Put into
        a server or client command, it runs whatever the player wrote, with the rights of the server.
        Refer to players by user id (`#%d` with `GetClientUserId()`) instead of by name, and never
        pass free text on. Reported is a variable holding client input that is an argument of a
        function of `functions` from the command's position on. A variable holds client input,
        within one function, once one of `sources` filled it, when it is a parameter of one of
        `forwards`, or once one of `propagators` wrote a variable holding client input into it.
    ]],
    options = {
        functions = {
            default = {
                "ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1", "ClientCommand:2",
                "FakeClientCommand:2", "FakeClientCommandEx:2",
            },
            description = "Functions that run a command, as Name:N with N the position of the command.",
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
    },
    check = function(file, ctx)
        local sink_specs, taint_specs = read_options(ctx.options)
        for _, function_node in ipairs(file:find(functions.kinds)) do
            local is_tainted = taint.analyze(function_node, taint_specs)
            for _, call in ipairs(function_node:find("call_expression")) do
                local name, is_method = calls.name(call)
                local first = not is_method and sink_specs[name] and sink_specs[name][1]
                for index, argument in ipairs(first and calls.arguments(call) or {}) do
                    if index >= first and is_tainted(argument) then
                        ctx:report(argument, ("`%s` holds client input and goes into %s()"):format(
                            argument:text(), name), { hint = "refer to players by `#userid`; do not pass free text" })
                    end
                end
            end
        end
    end,
}
