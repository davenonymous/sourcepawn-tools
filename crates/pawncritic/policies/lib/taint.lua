-- Copyright 2026 davenonymous

-- Client input inside one function: which variables hold text a player
-- controls, from the calls that fill them and the forwards that receive it,
-- passed on by the calls that copy or format text into another buffer.

local calls = pawn.library("calls")
local functions = pawn.library("functions")

-- The calls that fill a buffer with client input, with the buffer's position.
local default_sources = {
    "GetCmdArg:2", "GetCmdArgString:1", "GetClientName:2", "GetClientInfo:3", "GetEventString:3",
    "GetClientCookie:3",
}

-- The public forwards whose parameters hold client input, with their positions.
local default_forwards = { "OnClientSayCommand:3", "OnClientSayCommand_Post:3" }

-- The calls that pass text on into a buffer, with the buffer's position: the
-- buffer holds client input when any other argument does.
local default_propagators = { "Format:1", "FormatEx:1", "VFormat:1", "strcopy:1", "StrCat:1", "BuildPath:2" }

-- The name of a function node, or nil.
local function function_name(function_node)
    local name = function_node:child_by_field("name")
    return name and name:text()
end

-- The names of the parameters of a function at the given positions.
local function parameter_names(function_node, positions)
    local names = {}
    local parameters = function_node:child_by_field("parameters")
    local list = parameters and parameters:find("parameter_declaration") or {}
    for _, position in ipairs(positions) do
        local name = list[position] and list[position]:child_by_field("name")
        if name then
            names[#names + 1] = name:text()
        end
    end
    return names
end

-- The variable an argument names, or nil when it is no plain name.
local function variable_of(argument)
    if argument and argument:kind() == "identifier" then
        return argument:text()
    end
    return nil
end

-- What a call does to the tainted names, in document order: a list of
-- { start = byte, name = variable, tainted = boolean }.
local function events_of_call(call, specs, is_tainted_before)
    local events = {}
    local name, is_method = calls.name(call)
    local list = calls.arguments(call)
    local start = call:byte_range()
    for _, position in ipairs(not is_method and specs.sources[name] or {}) do
        local variable = variable_of(list[position])
        if variable then
            events[#events + 1] = { start = start, name = variable, tainted = true }
        end
    end
    for _, position in ipairs(specs.sanitizers[name] or {}) do
        local variable = variable_of(list[position])
        if variable then
            events[#events + 1] = { start = start, name = variable, tainted = false }
        end
    end
    for _, position in ipairs(not is_method and specs.propagators[name] or {}) do
        local output = variable_of(list[position])
        for index, argument in ipairs(list) do
            local input = variable_of(argument)
            if output and index ~= position and input and is_tainted_before(input, start) then
                events[#events + 1] = { start = start, name = output, tainted = true }
            end
        end
    end
    return events
end

-- Analyzes a function. `specs` holds parsed "Name:N" tables (see
-- calls.parse_positions): `sources`, `forwards`, `propagators` and
-- `sanitizers` (calls whose buffer at N no longer holds client input; may be
-- empty). Returns a function that tells whether an argument node holds client
-- input at its position.
local function analyze(function_node, specs)
    assert(functions.kind_set[function_node:kind()], "taint.analyze: the node is no function")
    local history = {}
    local function is_tainted_before(name, position)
        local tainted = false
        for _, event in ipairs(history[name] or {}) do
            if event.start < position then
                tainted = event.tainted
            end
        end
        return tainted
    end
    local function record(event)
        history[event.name] = history[event.name] or {}
        table.insert(history[event.name], event)
    end
    local function_start = function_node:byte_range()
    for _, name in ipairs(parameter_names(function_node, specs.forwards[function_name(function_node)] or {})) do
        record({ start = function_start, name = name, tainted = true })
    end
    for _, call in ipairs(function_node:find("call_expression")) do
        for _, event in ipairs(events_of_call(call, specs, is_tainted_before)) do
            record(event)
        end
    end
    return function(argument)
        local variable = variable_of(argument)
        return variable ~= nil and is_tainted_before(variable, argument:byte_range())
    end
end

return {
    default_sources = default_sources,
    default_forwards = default_forwards,
    default_propagators = default_propagators,
    analyze = analyze,
}
