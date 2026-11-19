-- Copyright 2026 davenonymous

-- The natives and stocks SourceMod's includes mark with `#pragma deprecated`,
-- with the advice the pragma gives.
local deprecated_functions = {
    FindDataMapOffs = "use FindDataMapInfo(), or HasEntProp() to check for existence",
    FindSendPropOffs = "use FindSendPropInfo(), or HasEntProp() to check for existence",
    FloatAdd = "use the `+` operator",
    FloatDiv = "use the `/` operator",
    FloatMod = "use the `%` operator",
    FloatMul = "use the `*` operator",
    FloatSub = "use the `-` operator",
    FormatUserLogText = "use the %L format specifier",
    GetAdmGroupImmunity = "use GetAdmGroupImmunityLevel()",
    GetClientAuthString = "use GetClientAuthId()",
    GetClientListening = "use GetListenOverride()",
    GetEntDataEnt = "use GetEntDataEnt2()",
    GetMaxClients = "use the MaxClients variable",
    GetSoundDuration = "it does not work and may crash",
    GuessSDKVersion = "use GetEngineVersion()",
    IsPlayerInGame = "use IsClientInGame()",
    IsSoundPrecached = "it always returns true",
    IsValidHandle = "it cannot tell a valid handle; keep handles in variables that `delete` resets",
    LoadMaps = "use ReadMapList()",
    SQL_ConnectEx = "use SQL_ConnectCustom()",
    SQL_QuoteString = "use SQL_EscapeString()",
    SearchForClients = "use FindTarget() or ProcessTargetString()",
    SetAdmGroupImmunity = "use SetAdmGroupImmunityLevel()",
    SetClientListening = "use SetListenOverride()",
    SetEntDataEnt = "use SetEntDataEnt2()",
    StrBreak = "use BreakString()",
    StrCompare = "use strcmp()",
    StrCopy = "use strcopy()",
    TF2_GetPlayerConditionFlags = "use TF2_IsPlayerInCondition()",
    TF2_GetPlayerResourceData = "use GetPlayerResourceEntity() and GetEntProp()",
    TF2_GetResourceEntity = "use GetPlayerResourceEntity()",
    TF2_SetPlayerResourceData = "use GetPlayerResourceEntity() and SetEntProp()",
}

-- Forwards SourceMod no longer calls or marks as deprecated.
local deprecated_forwards = {
    AskPluginLoad = "implement AskPluginLoad2()",
    OnLevelInit = "implement OnMapInit()",
    OnServerCfg = "implement OnConfigsExecuted()",
    TF2_OnGetHoliday = "it is no longer called; implement TF2_OnIsHolidayActive()",
}

return {
    name = "Modern::ProhibitDeprecatedFunctions",
    severity = 4,
    themes = { "core", "modern", "bugs" },
    summary = "SourceMod marks this function as deprecated",
    description = [[
        SourceMod's include files mark natives and stocks that should no longer be used with
        `#pragma deprecated`; the compiler warns about them. Some do not work any more, others have
        better replacements. Forwards that SourceMod no longer calls are reported where a plugin
        implements them.
    ]],
    on = {
        call_expression = function(node, ctx)
            local callee = node:child_by_field("function")
            local advice = callee:kind() == "identifier" and deprecated_functions[callee:text()]
            if advice then
                ctx:report(callee, ("`%s()` is deprecated; %s"):format(callee:text(), advice))
            end
        end,
        function_definition = function(node, ctx)
            local name = node:child_by_field("name")
            local advice = deprecated_forwards[name:text()]
            if advice then
                ctx:report(name, ("the forward `%s` is deprecated; %s"):format(name:text(), advice))
            end
        end,
    },
}
