-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

local parsed_options, functions_spec
local function read_functions(options)
    if parsed_options ~= options then
        parsed_options = options
        functions_spec = calls.parse_positions(options.functions, "functions")
    end
    return functions_spec
end

-- Whether a format argument is fixed text: a string literal, a name in
-- capitals (most likely a macro of one) or a `...` concatenation of those.
local function is_fixed_text(node)
    local kind = node:kind()
    if kind == "string_literal" then
        return true
    end
    if kind == "identifier" then
        return node:text():find("^[A-Z][A-Z0-9_]*$") ~= nil
    end
    if kind == "binary_expression" then
        local operator = node:child_by_field("operator")
        return operator ~= nil and operator:text() == "..." and is_fixed_text(node:child_by_field("left"))
            and is_fixed_text(node:child_by_field("right"))
    end
    return false
end

-- Format functions as the default of the option: SourceMod's, those of the
-- color libraries (morecolors, colors, multicolors) and of etf2l.inc.
local default_functions = {
    "PrintToChat:2", "PrintToChatAll:1", "PrintCenterText:2", "PrintCenterTextAll:1", "PrintHintText:2",
    "PrintHintTextToAll:1", "PrintToConsole:2", "PrintToConsoleAll:1", "PrintToServer:1", "ReplyToCommand:2",
    "ShowActivity:2", "ShowActivity2:3", "ShowActivityEx:3", "LogMessage:1", "LogError:1", "LogAction:3",
    "LogToFile:2", "LogToFileEx:2", "LogToGame:1", "Format:3", "FormatEx:3", "ThrowError:1", "ThrowNativeError:2",
    "SetFailState:1", "ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1", "ClientCommand:2",
    "FakeClientCommand:2", "FakeClientCommandEx:2", "KickClient:2", "KickClientEx:2", "ShowHudText:3",
    "ShowSyncHudText:3", "CPrintToChat:2", "CPrintToChatAll:1", "CPrintToChatEx:3", "CPrintToChatAllEx:2",
    "CReplyToCommand:2", "CShowActivity:2", "CShowActivity2:3", "CShowActivityEx:3", "ETF2L_PrintToChat:2",
    "ETF2L_PrintToChatEx:3", "ETF2L_PrintToChatAll:1", "ETF2L_PrintToChatAllEx:2", "ETF2L_PrintToTeam:2",
    "ETF2L_PrintToTeamEx:3", "ETF2L_PrintToAdmins:2", "ETF2L_PrintToAdminsEx:3", "ETF2L_ShowActivity:2",
    "ETF2L_PrintToServer:1", "ETF2L_PrintToConsole:2", "ETF2L_Reply:2", "ETF2L_ReplyEx:3", "ETF2L_LogDebug:1",
    "ETF2L_LogInfo:1", "ETF2L_LogWarning:1", "ETF2L_LogError:1",
}

return {
    name = "Security::ProhibitNonLiteralFormat",
    severity = 4,
    themes = { "bugs", "core", "security" },
    summary = "the format is a variable; a `%` in it is read as a format specifier",
    description = [[
        A format function reads every `%` of its format as a specifier. When the format is a variable,
        such as a player's name or chat message, a player who writes `%s` makes the call fail or
        print what it should not. Pass the text as an argument: `PrintToChat(client, "%s", text)`.
        Reported is the format argument of a function of `functions` when it is not fixed text and
        no argument follows it. Fixed text is a string literal, a name in capitals (most likely a
        macro) or a `...` concatenation of those.
    ]],
    options = {
        functions = {
            default = default_functions,
            description = "Format functions, as Name:N with N the position of the format.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            local name = calls.name(node)
            local positions = read_functions(ctx.options)[name]
            local position = positions and positions[1]
            local list = calls.arguments(node)
            local format = position and list[position]
            if format and #list == position and not is_fixed_text(format) then
                ctx:report(format, ("the format of %s() is `%s`, not fixed text"):format(name, format:text()), {
                    hint = ('pass it as an argument: "%%s", %s'):format(format:text()),
                })
            end
        end,
    },
}
