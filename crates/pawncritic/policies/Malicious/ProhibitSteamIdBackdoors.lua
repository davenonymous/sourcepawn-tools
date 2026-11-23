-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local functions = pawn.library("functions")

-- The shapes of a SteamID: STEAM_X:Y:Z, [U:1:N] and a 64-bit SteamID.
local steam_id_patterns = { "^STEAM_[0-5]:[01]:%d+$", "^%[U:1:%d+%]$", "^7656119%d%d%d%d%d%d%d%d%d%d$" }

local function is_steam_id(value)
    for _, pattern in ipairs(steam_id_patterns) do
        if value:find(pattern) then
            return true
        end
    end
    return false
end

-- Whether the function calls one of the functions that grant admin rights.
local function grants_admin(function_node, grant_functions)
    for _, call in ipairs(function_node:find(calls.call_kinds)) do
        if grant_functions[calls.name(call)] then
            return true
        end
    end
    return false
end

local parsed_options, grant_set
local function read_grants(options)
    if parsed_options ~= options then
        parsed_options = options
        grant_set = pawn.set(options.grant_functions)
    end
    return grant_set
end

return {
    name = "Malicious::ProhibitSteamIdBackdoors",
    severity = 5,
    themes = { "core", "malicious", "security" },
    summary = "a hard-coded SteamID in a function that grants admin rights",
    description = [[
        A function that compares a player with a SteamID written into the code and grants admin
        rights is a backdoor: the author keeps root access on every server that runs the plugin.
        Admins belong in the admin configuration of the server. Reported is each SteamID literal
        (`STEAM_X:Y:Z`, `[U:1:N]` or a 64-bit SteamID) in a function that also calls one of
        `grant_functions`.
    ]],
    options = {
        grant_functions = {
            default = {
                "SetUserFlagBits", "AddUserFlags", "SetUserAdmin", "CreateAdmin", "SetAdminFlag", "SetFlag",
                "AdminInheritGroup", "InheritGroup", "BindAdminIdentity", "BindIdentity", "SetAdmGroupAddFlag",
                "SetAdmGroupImmunityLevel", "SetAdminImmunityLevel", "SetImmunityLevel",
            },
            description = "Functions and methods that grant admin rights.",
        },
    },
    check = function(file, ctx)
        local grant_functions = read_grants(ctx.options)
        for _, function_node in ipairs(file:find(functions.kinds)) do
            local literals = {}
            for _, literal in ipairs(function_node:find("string_literal")) do
                local value = calls.string_value(literal)
                if value and is_steam_id(value) then
                    literals[#literals + 1] = literal
                end
            end
            if #literals > 0 and grants_admin(function_node, grant_functions) then
                for _, literal in ipairs(literals) do
                    ctx:report(literal, nil, { hint = "grant admin rights in the server's admin configuration" })
                end
            end
        end
    end,
}
