-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local paths = pawn.library("paths")

local parsed_options, file_specs, write_specs, extensions, directories
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        file_specs = calls.parse_positions(options.file_functions, "file_functions")
        write_specs = calls.parse_positions(options.write_functions, "write_functions")
        extensions, directories = {}, {}
        for _, extension in ipairs(options.code_extensions) do
            extensions[#extensions + 1] = extension:lower()
        end
        for _, directory in ipairs(options.code_directories) do
            directories[#directories + 1] = directory:lower():gsub("\\", "/")
        end
    end
    return file_specs, write_specs, extensions, directories
end

-- Whether a resolved path is a file of code: it has a code extension or lies
-- in a code directory.
local function is_code_path(path, extensions, directories)
    for _, extension in ipairs(extensions) do
        if pawn.ends_with(path, extension) then
            return true
        end
    end
    for _, directory in ipairs(directories) do
        if pawn.starts_with(path .. "/", directory) then
            return true
        end
    end
    return false
end

return {
    name = "Security::ProhibitCodeFileWrites",
    severity = 4,
    themes = { "core", "security" },
    summary = "the plugin writes, deletes or renames a file of code",
    description = [[
        Plugins, extensions and Metamod:Source plugins are code the server runs. A plugin that writes,
        deletes or renames them installs code nobody reviewed, removes other plugins or updates
        itself behind the owner's back. Reported is a literal path that reaches a writing function of
        `write_functions` (`OpenFile()` only with a mode of `w`, `a` or `+`), directly or as the
        template of the buffer passed, and that ends in one of `code_extensions` or lies in one of
        `code_directories` (below the game directory; `BuildPath(Path_SM, ...)` starts in
        `addons/sourcemod`).
    ]],
    options = {
        file_functions = {
            default = paths.default_file_functions,
            description = "Functions that take a path, as Name:N or Name:N,M with the paths' positions.",
        },
        write_functions = {
            default = paths.default_write_functions,
            description = "Those of file_functions that write, delete or rename, in the same form.",
        },
        code_extensions = {
            default = { ".smx", ".so", ".dll", ".dylib", ".vdf" },
            description = "File name endings of code, ignoring case.",
        },
        code_directories = {
            default = {
                "addons/sourcemod/plugins/", "addons/sourcemod/extensions/", "addons/sourcemod/bin/",
                "addons/metamod/", "bin/",
            },
            description = "Directories of code below the game directory, ignoring case.",
        },
    },
    check = function(file, ctx)
        local files, writers, extensions, directories = read_options(ctx.options)
        local reported = {}
        for _, path in ipairs(paths.path_literals(file, files)) do
            local resolved = paths.resolve(path.value, path.base)
            local is_write = path.call ~= nil and paths.writes(path.call, writers)
            local is_new = not reported[path.node:id()]
            if is_write and is_new and resolved and is_code_path(resolved, extensions, directories) then
                reported[path.node:id()] = true
                ctx:report(path.node, ("%s() changes the code file `%s`"):format(calls.name(path.call), path.value))
            end
        end
    end,
}
