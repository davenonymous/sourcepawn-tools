-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local paths = pawn.library("paths")

return {
    name = "Security::ProhibitPathTraversal",
    severity = 5,
    themes = { "core", "security" },
    summary = "the path climbs out of the game directory",
    description = [[
        A plugin's files belong in the game directory. A path whose `..` parts climb above it reaches
        the rest of the machine: other servers, the system, the owner's files. Checked are the literal
        paths passed to a file function of `file_functions`, directly or as the template of the buffer
        passed, and the path of every `BuildPath()`. A path starts in the game directory, the path of
        `BuildPath(Path_SM, ...)` in `addons/sourcemod`; a template that starts with a placeholder,
        such as `%s/../x`, starts in an unknown place and is not checked.
    ]],
    options = {
        file_functions = {
            default = paths.default_file_functions,
            description = "Functions that take a path, as Name:N or Name:N,M with the paths' positions.",
        },
    },
    check = function(file, ctx)
        local file_specs = calls.parse_positions(ctx.options.file_functions, "file_functions")
        local reported = {}
        for _, path in ipairs(paths.path_literals(file, file_specs)) do
            local _, escapes = paths.resolve(path.value, path.base)
            if escapes and not reported[path.node:id()] then
                reported[path.node:id()] = true
                ctx:report(path.node, ("the path `%s` climbs out of the game directory"):format(path.value))
            end
        end
    end,
}
