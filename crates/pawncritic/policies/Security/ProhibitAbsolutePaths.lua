-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local paths = pawn.library("paths")

return {
    name = "Security::ProhibitAbsolutePaths",
    severity = 4,
    themes = { "core", "security" },
    summary = "an absolute path reaches outside the game directory",
    description = [[
        File functions take paths relative to the game directory. An absolute path (one that starts
        with `/`, `\` or a drive letter such as `C:`) reaches anywhere on the machine. Checked are
        the literal paths passed to a file function of `file_functions`, directly or as the template
        of the buffer passed. `BuildPath()` always starts below the SourceMod directory and is not
        checked.
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
            local is_candidate = path.call and path.builder ~= "BuildPath" and not reported[path.node:id()]
            if is_candidate and paths.is_absolute(path.value) then
                reported[path.node:id()] = true
                ctx:report(path.node, ("the absolute path `%s` reaches outside the game directory"):format(path.value), {
                    hint = "use a path relative to the game directory, or BuildPath(Path_SM, ...)",
                })
            end
        end
    end,
}
