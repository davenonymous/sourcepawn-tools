-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

local parsed_options, functions_spec, commands
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        functions_spec = calls.parse_positions(options.functions, "functions")
        commands = {}
        for _, command in ipairs(options.commands) do
            commands[#commands + 1] = pawn.trim(command):lower():gsub("%s+", " ")
        end
    end
    return functions_spec, commands
end

-- The first listed command that a command of the line starts with, or nil.
-- Commands are separated by `;` and line breaks; whitespace counts as one
-- space and case is ignored.
local function listed_command(line, list)
    for command in line:gmatch("[^;\r\n]+") do
        local normalized = pawn.trim(command):lower():gsub("%s+", " ")
        for _, listed in ipairs(list) do
            if normalized == listed or pawn.starts_with(normalized, listed .. " ") then
                return listed
            end
        end
    end
    return nil
end

return {
    name = "Security::ProhibitServerControlCommands",
    severity = 4,
    themes = { "core", "security" },
    summary = "the plugin loads code or stops the server with a server command",
    description = [[
        A plugin that loads or unloads plugins, extensions or Metamod:Source plugins, sends RCON
        commands or shuts the server down takes decisions that belong to the server's owner. Plugin
        managers and updaters do this on purpose; anything else should not. Reported is a literal
        command (directly or as the template of the buffer passed) to a function of `functions`
        whose commands start with one of `commands`.
    ]],
    options = {
        functions = {
            default = { "ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1" },
            description = "Functions that run a server command, as Name:N with N the command's position.",
        },
        commands = {
            default = {
                "sm plugins load", "sm plugins unload", "sm plugins unload_all", "sm plugins reload",
                "sm plugins refresh", "sm exts load", "sm exts unload", "sm exts reload", "meta load",
                "meta unload", "plugin_load", "plugin_unload", "rcon", "sm_rcon", "quit", "exit", "_restart",
                "killserver",
            },
            description = "Commands, with their first words, ignoring case.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            local functions_spec, commands = read_options(ctx.options)
            local name, is_method = calls.name(node)
            local positions = not is_method and functions_spec[name]
            local list = calls.arguments(node)
            for _, position in ipairs(positions or {}) do
                for _, source in ipairs(list[position] and calls.literal_sources(list[position]) or {}) do
                    local command = listed_command(source.value, commands)
                    if command then
                        ctx:report(source.node, ("%s() runs `%s`"):format(name, command))
                    end
                end
            end
        end,
    },
}
