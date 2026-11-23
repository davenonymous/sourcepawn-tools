-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

local parsed_options, natives
local function read_natives(options)
    if parsed_options ~= options then
        parsed_options = options
        natives = calls.parse_positions(options.natives, "natives")
    end
    return natives
end

-- Reports each literal address a call passes to an address parameter.
local function report_literal_addresses(call, ctx)
    local name = calls.name(call)
    local positions = read_natives(ctx.options)[name]
    local list = calls.arguments(call)
    for _, position in ipairs(positions or {}) do
        for _, source in ipairs(list[position] and calls.literal_sources(list[position]) or {}) do
            local value = pawn.trim(source.value)
            if value ~= "" and not pawn.starts_with(value, "%") then
                ctx:report(source.node, ("%s() connects to `%s`, written into the code"):format(name, value), {
                    hint = "read the address from a ConVar or a config file",
                })
            end
        end
    end
end

return {
    name = "Security::ProhibitHardcodedEndpoints",
    severity = 4,
    themes = { "core", "security" },
    summary = "the plugin connects to an address written into the code",
    description = [[
        A plugin that sends data to a host, an IP address or a URL written into its code reports to
        its author: the owner cannot see or change where the data goes, and it is how stolen
        passwords and player data leave a server. Read addresses from a ConVar or a config file the
        owner controls. Reported is a literal address (directly, or as the template of the buffer
        passed when the template does not start with a placeholder) passed to a function of
        `natives`.
    ]],
    options = {
        natives = {
            default = {
                "SocketConnect:5", "SocketSendTo:4", "SteamWorks_CreateHTTPRequest:2", "System2HTTPRequest:2",
                "System2FTPRequest:2", "System2_GetPage:2", "System2_DownloadFile:2", "HTTPClient:1",
                "HTTPRequest:1",
            },
            description = "Functions and constructors that take an address, as Name:N with N its position.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            report_literal_addresses(node, ctx)
        end,
        new_expression = function(node, ctx)
            report_literal_addresses(node, ctx)
        end,
    },
}
