-- Copyright 2026 davenonymous

return {
    name = "Modern::RequireNewDecls",
    severity = 4,
    themes = { "core", "modern" },
    summary = "the file lacks `#pragma newdecls required`",
    description = [[
        `#pragma newdecls required` makes the compiler reject old-style declarations, so a plugin
        stays in modern SourcePawn once it is converted. Put it at the top of every plugin, after the
        includes. Include files are only checked with the option `check_includes`, since a pragma in
        an include also applies to the files that include it.
    ]],
    options = {
        check_includes = { default = false, description = "Also require the pragma in .inc files." },
    },
    check = function(file, ctx)
        if file.kind == "inc" and not ctx.options.check_includes then
            return
        end
        for _, directive in ipairs(file:directives()) do
            if directive.text:match("^#%s*pragma%s+newdecls%s+required") then
                return
            end
        end
        ctx:report({ line = 1 }, nil, { hint = "add `#pragma newdecls required` after the includes" })
    end,
}
