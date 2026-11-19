-- Copyright 2026 davenonymous

local declarations = pawn.library("declarations")

return {
    name = "Variables::RequireNarrowDeclarations",
    severity = 3,
    themes = { "core", "variables", "philosophy" },
    summary = "the variable is used only inside a nested block",
    description = [[
        A variable used only inside the body of one `if`, `else` or loop belongs to that body:
        declared there, its reader sees at once that nothing outside uses it, and it cannot be
        used by mistake after the block. A local variable whose every use lies inside one braced
        body of a condition or loop is reported, with the innermost such body. Only declarations
        without an initializer or with a constant one (a literal, `""`, `{...}`) are checked.

        A loop body is suggested only when no value has to survive from one pass to the next:
        the first use in the body is a plain assignment (`x = ...`) or an array passed to a call
        as the buffer it fills, followed by its size and nowhere else among the call's arguments
        (`GetClientName(client, name, sizeof(name))`), in a statement that always runs. Mind that an array declared in a loop body is filled with zeros on every pass; keep
        it outside a hot loop and silence this policy there with a `no critic` annotation.
    ]],
    check = function(file, ctx)
        local references = declarations.references_by_name(file)
        for _, declaration in ipairs(file:find("variable_declaration")) do
            local usage = declarations.analyze(declaration, references)
            if usage and usage.target then
                ctx:report(
                    declaration,
                    ("`%s` is used only inside the block at line %d"):format(usage.name, usage.target:line()),
                    {
                        hint = ("declare it inside that block, just before line %d"):format(
                            usage.target_statement:line()
                        ),
                    }
                )
            end
        end
    end,
}
