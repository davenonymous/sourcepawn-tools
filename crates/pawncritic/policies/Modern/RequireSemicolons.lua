-- Copyright 2026 davenonymous

return {
    name = "Modern::RequireSemicolons",
    severity = 4,
    themes = { "core", "modern" },
    summary = "the file lacks `#pragma semicolon 1`",
    description = [[
        Without `#pragma semicolon 1` the compiler accepts statements without a closing semicolon,
        and a line break may end a statement where the author did not mean it to. Put the pragma at
        the top of every plugin. Include files are only checked with the option `check_includes`.
    ]],
    options = {
        check_includes = { default = false, description = "Also require the pragma in .inc files." },
    },
    check = function(file, ctx)
        if file.kind == "inc" and not ctx.options.check_includes then
            return
        end
        for _, directive in ipairs(file:directives()) do
            if directive.text:match("^#%s*pragma%s+semicolon%s+1") then
                return
            end
        end
        ctx:report({ line = 1 }, nil, { hint = "add `#pragma semicolon 1` at the top" })
    end,
}
