-- Copyright 2026 davenonymous

local function is_max_clients(node)
    if node:kind() == "identifier" then
        return node:text() == "MaxClients"
    end
    if node:kind() == "call_expression" then
        return node:child_by_field("function"):text() == "GetMaxClients"
    end
    return false
end

local function operator_of(node)
    for _, child in ipairs(node:children()) do
        if child:field() == "operator" then
            return child:text()
        end
    end
end

return {
    name = "Bugs::RequireInclusiveMaxClients",
    severity = 4,
    themes = { "core", "bugs" },
    summary = "the loop stops before the last client; client indices run from 1 to MaxClients",
    description = [[
        Client indices run from 1 to `MaxClients` inclusive. A loop with `i < MaxClients` never
        reaches the last slot, a bug that only shows on a full server. Loop with
        `for (int client = 1; client <= MaxClients; client++)`.
    ]],
    on = {
        for_statement = function(node, ctx)
            local condition = node:child_by_field("condition")
            if not condition or condition:kind() ~= "binary_expression" then
                return
            end
            local operator = operator_of(condition)
            local left = condition:child_by_field("left")
            local right = condition:child_by_field("right")
            local stops_early = (operator == "<" and is_max_clients(right)) or (operator == ">" and is_max_clients(left))
            if stops_early then
                ctx:report(condition, nil, { hint = "compare with `<= MaxClients`" })
            end
        end,
    },
}
