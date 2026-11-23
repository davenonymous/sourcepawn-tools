-- Copyright 2026 davenonymous

-- Calls and string literals: the name and arguments of a call, the value of a
-- literal string, the "Name:N" specifications policies take as options, and
-- the literal a string buffer was filled with earlier in its function.

local functions = pawn.library("functions")

-- The functions that write a string into a buffer: the position of the
-- buffer and of the template or source string. A literal at the template
-- position is what the buffer holds afterwards (as far as a policy can tell).
local builders = {
    Format = { output = 1, template = 3 },
    FormatEx = { output = 1, template = 3 },
    VFormat = { output = 1, template = 3 },
    strcopy = { output = 1, template = 3 },
    StrCat = { output = 1, template = 3 },
    BuildPath = { output = 2, template = 4 },
}

-- The kinds of nodes that call something by name.
local call_kinds = { "call_expression", "new_expression" }

-- The name of what a call calls and whether it is a method: `Name(...)`
-- gives "Name", false; `x.Name(...)` gives "Name", true; `new Name(...)`
-- gives "Name", false. nil for anything else.
local function name(call)
    if call:kind() == "new_expression" then
        local class = call:child_by_field("class")
        return class and class:text(), false
    end
    local callee = call:child_by_field("function")
    if not callee then
        return nil
    end
    if callee:kind() == "identifier" then
        return callee:text(), false
    end
    if callee:kind() == "field_access" then
        local field = callee:child_by_field("field")
        return field and field:text(), true
    end
    return nil
end

-- The argument nodes of a call, comments left out.
local function arguments(call)
    local list = {}
    local node = call:child_by_field("arguments")
    if not node then
        return list
    end
    for _, child in ipairs(node:named_children()) do
        if child:kind() ~= "comment" then
            list[#list + 1] = child
        end
    end
    return list
end

local simple_escapes = { n = "\n", r = "\r", t = "\t", a = "\a", b = "\b", f = "\f", v = "\v", e = "\27" }

-- The text of one escape sequence such as `\x72`, `\114`, `\n` or `\"`.
local function unescape(sequence)
    local body = sequence:sub(2):gsub(";$", "")
    local hex = body:match("^[xX](%x+)$")
    if hex then
        return string.char(tonumber(hex, 16) % 256)
    end
    local decimal = body:match("^(%d+)$")
    if decimal then
        return string.char(tonumber(decimal) % 256)
    end
    return simple_escapes[body] or body
end

-- The value of a string literal or of a `...` concatenation of string
-- literals, with escape sequences decoded; nil for anything else.
local function string_value(node)
    local kind = node:kind()
    if kind == "binary_expression" then
        local operator = node:child_by_field("operator")
        if not operator or operator:text() ~= "..." then
            return nil
        end
        local left = string_value(node:child_by_field("left"))
        local right = left and string_value(node:child_by_field("right"))
        return right and left .. right
    end
    if kind ~= "string_literal" then
        return nil
    end
    local text = node:text()
    local start_byte = node:byte_range()
    local parts, position = {}, 2
    for _, escape in ipairs(node:find("escape_sequence")) do
        local escape_start, escape_end = escape:byte_range()
        parts[#parts + 1] = text:sub(position, escape_start - start_byte)
        parts[#parts + 1] = unescape(escape:text())
        position = escape_end - start_byte + 1
    end
    parts[#parts + 1] = text:sub(position, -2)
    return table.concat(parts)
end

-- Parses a list of "Name:N" or "Name:N,M" into a table mapping each name to
-- the list of its positions. A malformed entry raises an error that names it.
local function parse_positions(list, option)
    local specs = {}
    for _, entry in ipairs(list) do
        local entry_name, numbers = entry:match("^%s*([%w_]+)%s*:%s*([%d,%s]+)$")
        if not entry_name then
            error(("option `%s`: `%s` is not of the form Name:N"):format(option, entry), 0)
        end
        local positions = {}
        for number in numbers:gmatch("%d+") do
            positions[#positions + 1] = tonumber(number)
        end
        specs[entry_name] = positions
    end
    return specs
end

-- Parses an option list once per options table: the options of a policy's
-- state never change, so the result is kept for as long as the list is the
-- same table.
local function cached(parse)
    local cache = setmetatable({}, { __mode = "k" })
    return function(list, option)
        local parsed = cache[list]
        if not parsed then
            parsed = parse(list, option)
            cache[list] = parsed
        end
        return parsed
    end
end

-- The function a node lies in, or nil at the top level.
local function enclosing_function(node)
    return node:ancestor(functions.kinds)
end

-- The literal values a call argument stands for: the argument itself when it
-- is a literal, or for a variable the template literals of the builder calls
-- (Format, FormatEx, VFormat, strcopy, StrCat, BuildPath) that fill it earlier
-- in the same function. A list of { node = literal, value = string, builder =
-- name or nil, builder_call = the builder's call or nil }.
local function literal_sources(argument)
    local value = string_value(argument)
    if value then
        return { { node = argument, value = value } }
    end
    local sources = {}
    if argument:kind() ~= "identifier" then
        return sources
    end
    local scope = enclosing_function(argument)
    if not scope then
        return sources
    end
    local variable = argument:text()
    local argument_start = argument:byte_range()
    for _, call in ipairs(scope:find("call_expression")) do
        local call_start = call:byte_range()
        local builder_name, is_method = name(call)
        local builder = not is_method and builders[builder_name]
        if call_start < argument_start and builder then
            local list = arguments(call)
            local output, template = list[builder.output], list[builder.template]
            local template_value = template and string_value(template)
            if output and output:text() == variable and template_value then
                sources[#sources + 1] =
                    { node = template, value = template_value, builder = builder_name, builder_call = call }
            end
        end
    end
    return sources
end

return {
    builders = builders,
    call_kinds = call_kinds,
    name = name,
    arguments = arguments,
    string_value = string_value,
    parse_positions = cached(parse_positions),
    literal_sources = literal_sources,
}
