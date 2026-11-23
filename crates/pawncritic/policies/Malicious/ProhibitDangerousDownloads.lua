-- Copyright 2026 davenonymous

local calls = pawn.library("calls")

local parsed_options, functions_spec, extensions, directories
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        functions_spec = calls.parse_positions(options.functions, "functions")
        extensions = {}
        for _, extension in ipairs(options.extensions) do
            extensions[#extensions + 1] = extension:lower()
        end
        directories = {}
        for _, directory in ipairs(options.directories) do
            directories[#directories + 1] = directory:lower():gsub("\\", "/")
        end
    end
    return functions_spec, extensions, directories
end

-- Why a download path is dangerous, or nil.
local function danger(value, extensions, directories)
    local path = value:lower():gsub("\\", "/"):gsub("^%./", "")
    for _, extension in ipairs(extensions) do
        if pawn.ends_with(path, extension) then
            return ("a `%s` file"):format(extension)
        end
    end
    for _, directory in ipairs(directories) do
        if pawn.starts_with(path, directory) then
            return ("a file in `%s`"):format(directory)
        end
    end
    return nil
end

return {
    name = "Malicious::ProhibitDangerousDownloads",
    severity = 5,
    themes = { "core", "malicious", "security" },
    summary = "the plugin makes players download a config file or a program",
    description = [[
        Every file in the downloads table lands in the player's game directory. A config file there
        can replace the player's own (`autoexec.cfg`, `config.cfg`) and run commands on their game,
        and a program has no place among game files at all. Reported is a literal path passed to a
        function of `functions` (directly or as the template of the buffer passed) that ends in one
        of `extensions` or starts with one of `directories`.
    ]],
    options = {
        functions = {
            default = { "AddFileToDownloadsTable:1" },
            description = "Functions that add a file to the downloads, as Name:N with N the path's position.",
        },
        extensions = {
            default = {
                ".cfg", ".exe", ".dll", ".so", ".dylib", ".bat", ".cmd", ".com", ".vbs", ".ps1", ".sh", ".scr",
                ".jar", ".lnk",
            },
            description = "File name endings that are never downloaded, ignoring case.",
        },
        directories = {
            default = { "cfg/", "bin/" },
            description = "Directories of the game no download goes to, ignoring case.",
        },
    },
    on = {
        call_expression = function(node, ctx)
            local functions_spec, extensions, directories = read_options(ctx.options)
            local name, is_method = calls.name(node)
            local positions = not is_method and functions_spec[name]
            local list = calls.arguments(node)
            for _, position in ipairs(positions or {}) do
                for _, source in ipairs(list[position] and calls.literal_sources(list[position]) or {}) do
                    local reason = danger(source.value, extensions, directories)
                    if reason then
                        ctx:report(source.node, ("players would download %s"):format(reason))
                    end
                end
            end
        end,
    },
}
