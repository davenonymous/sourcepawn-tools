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

return {
    name = "CodeLayout::RequireTabIndentation",
    severity = 3,
    themes = { "core", "layout", "tidy" },
    summary = "indentation with spaces; indent with tabs",
    description = [[
        Code is indented with tabs, one per level, so everyone sees the indentation at the width
        they set. Spaces may follow the tabs only to align a continued line with something on the
        line above, never to indent. This policy reports a line that starts a statement, a
        declaration, a comment, a directive or a closing brace with spaces in its indentation, and
        any line whose indentation has a space before a tab.
    ]],
    check = function(file, ctx)
        local starts_line = {}
        for _, node in ipairs(file:find(line_kinds)) do
            starts_line[(node:byte_range())] = true
        end
        local first_token_of_line = {}
        for _, token in ipairs(file:tokens()) do
            if not first_token_of_line[token.line] then
                first_token_of_line[token.line] = token
            end
        end
        for number, text in file:lines() do
            local token = first_token_of_line[number]
            local indentation = text:match("^[ \t]*")
            if token and token.column == #indentation + 1 then
                local is_unit = starts_line[token.start_byte] or token.text == "}" or token.text == "else"
                    or token.kind == "line_comment" or token.kind == "block_comment" or token.kind == "directive"
                if indentation:find(" \t") then
                    ctx:report({ line = number, end_column = #indentation + 1 }, "a space before a tab in the indentation")
                elseif is_unit and indentation:find(" ") then
                    ctx:report({ line = number, end_column = #indentation + 1 })
                end
            end
        end
    end,
}
