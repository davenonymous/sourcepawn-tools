-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

-- Whether a number is the code of a printable ASCII character.
local function is_printable(code)
    return code ~= nil and code >= 32 and code <= 126
end

-- The type a declared variable has, as written, or nil.
local function declared_type(declaration)
    if declaration:kind() == "old_variable_declaration" then
        local old_type = declaration:child_by_field("type")
        return old_type and old_type:text():gsub("%s*:$", "")
    end
    local statement = declaration:parent()
    local type_node = statement and statement:child_by_field("type")
    return type_node and type_node:text()
end

-- The text an array literal spells, or nil: it must initialize a char array
-- and hold at least `min_length` items, all integers of printable characters.
local function numeric_text(array, min_length)
    local declaration = array:parent()
    if array:field() ~= "initialValue" or not declaration then
        return nil
    end
    local type_text = declared_type(declaration)
    if type_text ~= "char" and type_text ~= "String" then
        return nil
    end
    local characters = {}
    for _, item in ipairs(array:named_children()) do
        if item:kind() ~= "comment" then
            local code = item:kind() == "int_literal" and tonumber(item:text())
            if not is_printable(code) then
                return nil
            end
            characters[#characters + 1] = string.char(code)
        end
    end
    return #characters >= min_length and table.concat(characters) or nil
end

-- The number of numeric escapes (\xNN, \NNN) of printable characters.
local function printable_escapes(literal)
    local count = 0
    for _, escape in ipairs(literal:find("escape_sequence")) do
        local body = escape:text():sub(2):gsub(";$", "")
        local hex, decimal = body:match("^[xX](%x+)$"), body:match("^(%d+)$")
        local code = hex and tonumber(hex, 16) or decimal and tonumber(decimal)
        if is_printable(code) then
            count = count + 1
        end
    end
    return count
end

-- Whether a value is a long run of base64 or hex digits.
local function is_blob(value, min_length)
    return #value >= min_length and (value:find("^[%w+/]+=*$") ~= nil)
end

return {
    name = "Malicious::ProhibitObfuscatedStrings",
    severity = 5,
    themes = { "core", "malicious", "security" },
    summary = "text is hidden from the reader",
    description = [[
        Plain text written so that a reader cannot see it is how malicious plugins hide commands,
        ConVar names and addresses from a review. Reported are a char array initialized with at
        least `min_length` integers that are all printable characters (`char cmd[] = { 114, 99,
        ... }`), a string literal with at least `min_escapes` escapes (`\x72`, `\114`) of printable
        characters, and a string literal of at least `min_blob_length` characters that are all
        base64 or hex digits.
    ]],
    options = {
        min_length = { default = 8, description = "Printable integers that make a char array text." },
        min_escapes = { default = 4, description = "Escapes of printable characters that make a string hidden." },
        min_blob_length = { default = 64, description = "Length of a base64 or hex string that counts as a blob." },
    },
    on = {
        array_literal = function(node, ctx)
            local text = numeric_text(node, ctx.options.min_length)
            if text then
                ctx:report(node, "the text is written as character codes", {
                    hint = ("write it as a string: \"%s\""):format(text),
                })
            end
        end,
        string_literal = function(node, ctx)
            if printable_escapes(node) >= ctx.options.min_escapes then
                ctx:report(node, "printable characters are written as escapes", {
                    hint = ("write them plainly: \"%s\""):format(calls.string_value(node)),
                })
                return
            end
            local value = calls.string_value(node)
            if value and is_blob(value, ctx.options.min_blob_length) then
                ctx:report(node, "a long base64 or hex string hides what it holds")
            end
        end,
    },
}
