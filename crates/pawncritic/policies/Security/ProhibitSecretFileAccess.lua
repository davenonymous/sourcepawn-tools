-- Copyright 2026 davenonymous

local calls = pawn.library("calls")
local paths = pawn.library("paths")

local parsed_options, file_specs, secret_names
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        file_specs = calls.parse_positions(options.file_functions, "file_functions")
        secret_names = {}
        for _, name in ipairs(options.secret_files) do
            secret_names[name:lower()] = true
        end
    end
    return file_specs, secret_names
end

-- The last part of a path, lower case.
local function file_name(value)
    return (value:lower():gsub("\\", "/"):match("([^/]*)$"))
end

return {
    name = "Security::ProhibitSecretFileAccess",
    severity = 4,
    themes = { "core", "security" },
    summary = "the plugin opens a file with credentials",
    description = [[
        `databases.cfg` holds the database passwords, the admin files say who controls the server,
        `server.cfg` usually holds the RCON password. A plugin reads them only to steal or change
        them; SourceMod gives plugins what they need through its API (`SQL_Connect()`, the admin
        functions). Reported is a literal path passed to a file function of `file_functions`,
        directly or as the template of the buffer passed, or the path of a `BuildPath()`, whose
        file name is one of `secret_files`.
    ]],
    options = {
        file_functions = {
            default = paths.default_file_functions,
            description = "Functions that take a path, as Name:N or Name:N,M with the paths' positions.",
        },
        secret_files = {
            default = {
                "databases.cfg", "admins.cfg", "admins_simple.ini", "admin_groups.cfg", "admin_overrides.cfg",
                "core.cfg", "server.cfg",
            },
            description = "File names that hold credentials or admin rights, ignoring case.",
        },
    },
    check = function(file, ctx)
        local files, secrets = read_options(ctx.options)
        local reported = {}
        for _, path in ipairs(paths.path_literals(file, files)) do
            local name = file_name(path.value)
            if secrets[name] and not reported[path.node:id()] then
                reported[path.node:id()] = true
                ctx:report(path.node, ("the plugin opens `%s`"):format(name), {
                    hint = "use SourceMod's API (SQL_Connect, the admin functions) instead of the file",
                })
            end
        end
    end,
}
