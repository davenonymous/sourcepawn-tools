-- Copyright 2026 davenonymous

-- The tags of old Pawn, such as `Float:`.

-- The name of a tag as written, `Float:` or `Float :`, without its colon.
local function name(tag)
    if pawn.ends_with(tag, ":") then
        return pawn.trim(tag:sub(1, -2))
    end
    return tag
end

return {
    name = name,
}
