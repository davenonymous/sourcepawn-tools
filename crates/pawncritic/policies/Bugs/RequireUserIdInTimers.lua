-- Copyright 2026 davenonymous

-- The position of the data argument of each delayed callback.
local data_argument = {
    CreateTimer = 3,
    RequestFrame = 2,
}

local function argument_nodes(call)
    local arguments = {}
    for _, child in ipairs(call:child_by_field("arguments"):named_children()) do
        if child:kind() ~= "comment" then
            arguments[#arguments + 1] = child
        end
    end
    return arguments
end

-- The client names as a set, built once: the options of a policy's state
-- never change, so the set is kept for as long as ctx.options is the same
-- table.
local names_options, client_names
local function client_name_set(options)
    if names_options ~= options then
        names_options = options
        client_names = pawn.set(options.client_names)
    end
    return client_names
end

return {
    name = "Bugs::RequireUserIdInTimers",
    severity = 3,
    themes = { "core", "bugs" },
    summary = "a client index passed to a delayed callback; pass the user id instead",
    description = [[
        By the time a timer or frame callback runs, the client may have left and another player may
        have the same index. Pass `GetClientUserId(client)` and turn it back with
        `GetClientOfUserId(userid)` in the callback, which gives 0 when the player is gone. For
        entities, pass `EntIndexToEntRef(entity)` and use `EntRefToEntIndex()`. The option
        `client_names` lists the variable names taken for client indices.
    ]],
    options = {
        client_names = {
            default = { "client", "iClient", "target", "attacker", "victim", "player", "owner", "builder" },
            description = "Names of variables that hold client indices.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            local callee = node:child_by_field("function")
            local position = callee:kind() == "identifier" and data_argument[callee:text()]
            if not position then
                return
            end
            local argument = argument_nodes(node)[position]
            if not argument or argument:kind() ~= "identifier" then
                return
            end
            if client_name_set(ctx.options)[argument:text()] then
                ctx:report(argument, ("`%s` passes a client index to %s(); pass `GetClientUserId(%s)`"):format(
                    argument:text(), callee:text(), argument:text()))
            end
        end,
    },
}
