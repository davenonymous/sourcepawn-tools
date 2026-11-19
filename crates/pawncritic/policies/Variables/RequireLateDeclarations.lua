-- Copyright 2026 davenonymous

local declarations = pawn.library("declarations")

local function plural(count, word)
    return count == 1 and ("%d %s"):format(count, word) or ("%d %ss"):format(count, word)
end

return {
    name = "Variables::RequireLateDeclarations",
    severity = 3,
    themes = { "core", "variables", "philosophy" },
    summary = "the variable is declared long before it is used",
    description = [[
        A variable declared where it is first needed tells its reader what it is for; declared
        early, it has to be kept in mind across code that does not touch it. Declare it right
        where it is first used, such as `char name[64];` on the line before the
        `GetClientName()` that fills it. A local variable is reported when at least
        `min_statements` statements (default 1) that do not use it stand between its
        declaration and its first use in the same block. Other declarations in between do not
        count, so a group of declarations is reported only for the variables used after other
        work. Only declarations without an initializer or with a constant one (a literal, `""`,
        `{...}`) are checked, since moving them never changes what the code does. A variable
        used only inside one braced `if`, `else` or loop body is left to
        `Variables::RequireNarrowDeclarations`, which tells to declare it inside that block.
    ]],
    options = {
        min_statements = {
            default = 1,
            description = "How many statements between declaration and first use are reported.",
        },
    },
    check = function(file, ctx)
        local references = declarations.references_by_name(file)
        for _, declaration in ipairs(file:find("variable_declaration")) do
            local usage = declarations.analyze(declaration, references)
            if usage and usage.target == nil and usage.gap >= ctx.options.min_statements then
                ctx:report(
                    declaration,
                    ("`%s` is declared %s before its first use"):format(usage.name, plural(usage.gap, "statement")),
                    {
                        hint = ("declare it just before line %d, where it is first used"):format(
                            usage.first_statement:line()
                        ),
                    }
                )
            end
        end
    end,
}
