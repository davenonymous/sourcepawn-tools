-- Copyright 2026 davenonymous

-- File paths in the code: the literal paths that reach a file function,
-- directly or through a buffer filled with BuildPath, Format, FormatEx,
-- VFormat, strcopy or StrCat, and where such a path leads.

local calls = pawn.library("calls")

-- The file functions and the positions of their path arguments, as the
-- default of a policy option.
local default_file_functions = {
    "OpenFile:1", "DeleteFile:1", "RenameFile:1,2", "FileExists:1", "DirExists:1", "CreateDirectory:1",
    "RemoveDir:1", "OpenDirectory:1", "FileSize:1", "GetFileTime:1", "SetFilePermissions:1", "FileToKeyValues:2",
    "KeyValuesToFile:2", "ImportFromFile:1", "ExportToFile:1", "LogToFile:1", "LogToFileEx:1",
    "SteamWorks_WriteHTTPResponseBodyToFile:2", "SetOutputFile:1",
}

-- The file functions that write, delete or rename, as the default of a policy
-- option. OpenFile writes only when its mode does; see opens_for_writing.
local default_write_functions = {
    "OpenFile:1", "DeleteFile:1", "RenameFile:1,2", "CreateDirectory:1", "RemoveDir:1", "KeyValuesToFile:2",
    "ExportToFile:1", "LogToFile:1", "LogToFileEx:1", "SteamWorks_WriteHTTPResponseBodyToFile:2", "SetOutputFile:1",
}

-- Where the path of BuildPath(Path_SM, ...) starts, below the game directory.
local sourcemod_prefix = "addons/sourcemod/"

-- Whether a call of OpenFile opens for writing: its mode is a literal with
-- `w`, `a` or `+`, or not a literal (and so may be one).
local function opens_for_writing(call)
    local mode = calls.arguments(call)[2]
    local value = mode and calls.string_value(mode)
    return value == nil or value:find("[wa+]") ~= nil
end

-- Whether a call of a file function writes: every write function does,
-- OpenFile only with a writing mode.
local function writes(call, write_specs)
    local function_name, is_method = calls.name(call)
    if not write_specs[function_name] then
        return false
    end
    return function_name ~= "OpenFile" or is_method or opens_for_writing(call)
end

-- Where a path from a literal starts: "" for the game directory,
-- `sourcemod_prefix` for BuildPath(Path_SM, ...), nil when unknown (a
-- template that starts with a placeholder, or BuildPath with another type).
local function base_of(source)
    if pawn.starts_with(source.value, "%") then
        return nil
    end
    if source.builder ~= "BuildPath" then
        return ""
    end
    local path_type = calls.arguments(source.builder_call)[1]
    if path_type and path_type:text() == "Path_SM" then
        return sourcemod_prefix
    end
    return nil
end

-- Every path literal of a file: for each, { node = literal, value = decoded
-- text, base = see base_of, builder = the name of the builder the literal is
-- the template of or nil, call = the file function call it reaches or nil }.
-- A literal of BuildPath appears once without `call` and once for each file
-- function it reaches.
local function path_literals(file, file_specs)
    local found = {}
    for _, call in ipairs(file:find(calls.call_kinds)) do
        local function_name, is_method = calls.name(call)
        local positions = file_specs[function_name]
        local list = calls.arguments(call)
        for _, position in ipairs(positions or {}) do
            local argument = list[position]
            for _, source in ipairs(argument and calls.literal_sources(argument) or {}) do
                source.base, source.call = base_of(source), call
                found[#found + 1] = source
            end
        end
        if function_name == "BuildPath" and not is_method and list[4] then
            local value = calls.string_value(list[4])
            if value then
                local source = { node = list[4], value = value, builder = "BuildPath", builder_call = call }
                source.base = base_of(source)
                found[#found + 1] = source
            end
        end
    end
    return found
end

-- Where a path leads from its base: the normalized path (lower case, `/`
-- between parts, no `.` and `..` parts) and whether it climbs above the game
-- directory. Absolute paths and unknown bases give nil, false.
local function resolve(value, base)
    if base == nil or value:find("^[/\\]") or value:find("^%a:") then
        return nil, false
    end
    local parts = {}
    for part in (base .. value):lower():gsub("\\", "/"):gmatch("[^/]+") do
        if part == ".." then
            if #parts == 0 then
                return nil, true
            end
            parts[#parts] = nil
        elseif part ~= "." then
            parts[#parts + 1] = part
        end
    end
    return table.concat(parts, "/"), false
end

-- Whether a path is absolute: it starts with `/`, `\` or a drive letter.
local function is_absolute(value)
    return value:find("^[/\\]") ~= nil or value:find("^%a:") ~= nil
end

return {
    default_file_functions = default_file_functions,
    default_write_functions = default_write_functions,
    writes = writes,
    path_literals = path_literals,
    resolve = resolve,
    is_absolute = is_absolute,
}
