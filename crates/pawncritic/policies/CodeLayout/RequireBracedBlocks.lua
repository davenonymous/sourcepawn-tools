-- Copyright 2026 davenonymous

local function is_block(node)
    return node:kind() == "block"
end

local function report_body(body, keyword, ctx)
    if body and not is_block(body) then
        ctx:report(body, ("the body of `%s` is not in braces"):format(keyword), { hint = "wrap it in `{ ... }`" })
    end
end

return {
    name = "CodeLayout::RequireBracedBlocks",
    severity = 3,
    themes = { "core", "layout", "tidy" },
    summary = "the body of a condition or loop is not in braces",
    description = [[
        Every `if`, `else`, `for`, `while`, `do` and, with `check_switch_cases`, every `case` and
        `default` puts its body in braces, even a single statement. A statement added later then
        cannot fall out of the condition, and every body looks the same. `else if` needs no braces
        around the inner `if`.
    ]],
    options = {
        check_switch_cases = { default = true, description = "Also require braces around case bodies." },
    },
    on = {
        condition_statement = function(node, ctx)
            report_body(node:child_by_field("truePath"), "if", ctx)
            local false_path = node:child_by_field("falsePath")
            if false_path and false_path:kind() ~= "condition_statement" then
                report_body(false_path, "else", ctx)
            end
        end,
        for_statement = function(node, ctx)
            report_body(node:child_by_field("body"), "for", ctx)
        end,
        while_statement = function(node, ctx)
            report_body(node:child_by_field("body"), "while", ctx)
        end,
        do_while_statement = function(node, ctx)
            report_body(node:child_by_field("body"), "do", ctx)
        end,
        switch_case = function(node, ctx)
            if ctx.options.check_switch_cases then
                local keyword = node:text():match("^default") and "default" or "case"
                report_body(node:child_by_field("body"), keyword, ctx)
            end
        end,
    },
}
