-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

local parsed_options, functions_spec, command_set
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        functions_spec = calls.parse_positions(options.functions, "functions")
        command_set = pawn.set(options.commands)
    end
    return functions_spec, command_set
end

-- The first word of each command in a command line, lower case: commands
-- are separated by `;` and line breaks.
local function command_words(line)
    local words = {}
    for command in line:gmatch("[^;\r\n]+") do
        local word = command:match("^%s*\"?([^%s\"]+)")
        if word then
            words[#words + 1] = word:lower()
        end
    end
    return words
end

return {
    name = "Malicious::ProhibitClientSlowhack",
    severity = 5,
    themes = { "core", "malicious", "security" },
    summary = "the plugin runs a command on a player's game that changes or leaves it",
    description = [[
        `ClientCommand()` runs a command in the player's own game. Binding keys, writing or running
        config files, changing download and upload settings or connecting to another server there is
        "slowhacking": it harms the player beyond this server or redirects them to another one.
        Reported is a literal command (directly or as the template of the buffer passed) whose
        commands start with a word in `commands`.
    ]],
    options = {
        functions = {
            default = { "ClientCommand:2" },
            description = "Functions that run a command on a client, as Name:N with N the command's position.",
        },
        commands = {
            default = {
                "bind", "unbind", "unbindall", "alias", "connect", "redirect", "writecfg", "exec",
                "cl_downloadfilter", "cl_allowupload", "cl_allowdownload", "motdfile", "motd_write", "con_logfile",
                "host_writeconfig", "quit", "exit",
            },
            description = "Client commands a plugin must not run, lower case.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            local functions_spec, commands = read_options(ctx.options)
            local name, is_method = calls.name(node)
            local positions = not is_method and functions_spec[name]
            if not positions then
                return
            end
            local list = calls.arguments(node)
            for _, position in ipairs(positions) do
                for _, source in ipairs(list[position] and calls.literal_sources(list[position]) or {}) do
                    for _, word in ipairs(command_words(source.value)) do
                        if commands[word] then
                            ctx:report(source.node, ("%s() runs `%s` on the player's game"):format(name, word))
                            break
                        end
                    end
                end
            end
        end,
    },
}
