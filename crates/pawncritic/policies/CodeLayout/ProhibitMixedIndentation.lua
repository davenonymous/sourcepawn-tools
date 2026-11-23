-- Copyright 2026 davenonymous

-- The nodes that start a line of their own when laid out: their first line
-- is indented, while the lines after it continue them.
local line_kinds = {
    "block", "variable_declaration_statement", "old_variable_declaration_statement", "for_statement",
    "while_statement", "do_while_statement", "break_statement", "continue_statement", "condition_statement",
    "switch_statement", "switch_case", "return_statement", "delete_statement", "expression_statement",
    "function_definition", "function_declaration", "enum", "enum_entry", "enum_struct", "enum_struct_field",
    "enum_struct_method", "typedef", "typeset", "functag", "funcenum", "methodmap", "methodmap_alias",
    "methodmap_native", "methodmap_native_constructor", "methodmap_native_destructor", "methodmap_method",
    "methodmap_method_constructor", "methodmap_method_destructor", "methodmap_property",
    "methodmap_property_alias", "methodmap_property_native", "methodmap_property_method", "struct",
    "struct_field", "struct_declaration", "struct_field_value", "global_variable_declaration",
    "old_global_variable_declaration", "assertion", "alias_declaration", "alias_assignment",
}

-- How many offending line numbers the hint lists before it counts the rest.
local listed_lines = 10

-- The indented lines whose first token starts the line: their number, their
-- indentation and whether they start a unit (a statement, a declaration, a
-- comment, a directive, a `}` or an `else`) rather than continue one.
local function indented_lines(file)
    local starts_unit = {}
    for _, node in ipairs(file:find(line_kinds)) do
        starts_unit[(node:byte_range())] = true
    end
    local first_token_of_line = {}
    for _, token in ipairs(file:tokens()) do
        if not first_token_of_line[token.line] then
            first_token_of_line[token.line] = token
        end
    end
    local found = {}
    for number, text in file:lines() do
        local token = first_token_of_line[number]
        local indentation = text:match("^[ \t]*")
        if token and #indentation > 0 and token.column == #indentation + 1 then
            local is_unit = starts_unit[token.start_byte] or token.text == "}" or token.text == "else"
                or token.kind == "line_comment" or token.kind == "block_comment" or token.kind == "directive"
            found[#found + 1] = { number = number, indentation = indentation, is_unit = is_unit }
        end
    end
    return found
end

-- The style most unit lines are indented with, tabs on a tie; nil when no
-- unit line is indented.
local function majority_style(lines)
    local tabs, spaces = 0, 0
    for _, line in ipairs(lines) do
        if line.is_unit and line.indentation:sub(1, 1) == "\t" then
            tabs = tabs + 1
        elseif line.is_unit then
            spaces = spaces + 1
        end
    end
    if tabs + spaces == 0 then
        return nil
    end
    return spaces > tabs and "spaces" or "tabs"
end

local function is_indented_with(style, line)
    if style == "spaces" then
        return not line.indentation:find("\t")
    end
    local is_space_before_tab = line.indentation:find(" \t") ~= nil
    return not is_space_before_tab and not (line.is_unit and line.indentation:find(" "))
end

-- "line 9", "lines 9 and 14", "lines 9, 14 and 20", "lines 1, ..., 10 and 5 more".
local function line_list(numbers)
    if #numbers == 1 then
        return "line " .. numbers[1]
    end
    local shown = {}
    for index = 1, math.min(#numbers, listed_lines) do
        shown[index] = tostring(numbers[index])
    end
    local rest = #numbers - #shown
    local last = rest > 0 and (rest .. " more") or table.remove(shown)
    return "lines " .. table.concat(shown, ", ") .. " and " .. last
end

local function required_style(option, lines)
    if option == "auto" then
        return majority_style(lines)
    end
    if option == "tabs" or option == "spaces" then
        return option
    end
    error(('the option indent_style is "%s", but must be "auto", "tabs" or "spaces"'):format(option), 0)
end

return {
    name = "CodeLayout::ProhibitMixedIndentation",
    severity = 4,
    themes = { "core", "layout", "tidy" },
    summary = "mixed indentation; indent with either tabs or spaces, not both",
    description = [[
        Every line of a file is indented the same way: either with tabs, one per level, with spaces
        after the tabs only to align a continued line with something on the line above, or with
        spaces alone. Mixed indentation looks right only at the tab width of its author. By default
        the file decides: the style most lines that start a statement, a declaration, a comment, a
        directive, a closing brace or an `else` are indented with, tabs on a tie. This policy
        reports the first line that breaks the style, once per file, and lists the others in the
        hint. In a file indented with tabs, a line breaks it when its indentation has a space
        before a tab, or when it starts one of these and its indentation holds a space; in a file
        indented with spaces, when its indentation holds a tab.
    ]],
    options = {
        indent_style = {
            default = "auto",
            description = '"auto" (the style most lines use), "tabs" or "spaces".',
        },
    },
    check = function(file, ctx)
        local lines = indented_lines(file)
        local style = required_style(ctx.options.indent_style, lines)
        if style == nil then
            return
        end
        local offending = {}
        for _, line in ipairs(lines) do
            if not is_indented_with(style, line) then
                offending[#offending + 1] = line
            end
        end
        if #offending == 0 then
            return
        end
        local numbers = {}
        for index, line in ipairs(offending) do
            numbers[index] = line.number
        end
        local verb = #offending == 1 and "differs" or "differ"
        local count = #offending == 1 and "1 line is" or (#offending .. " lines are")
        local first = offending[1]
        ctx:report(
            { line = first.number, end_column = #first.indentation + 1 },
            ("mixed indentation; %s not indented with %s"):format(count, style),
            {
                hint = ('%s %s; run `pawntidy --write %s` with indent_style = "%s" in .pawntidy.toml'):format(
                    line_list(numbers), verb, file.path, style),
            }
        )
    end,
}
