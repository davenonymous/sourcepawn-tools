-- Copyright 2026 davenonymous

local tags = pawn.library("tags")

local replacements = {
    Float = "float",
    String = "char",
    bool = "bool",
    _ = "int",
}

return {
    name = "Modern::ProhibitOldTypeTags",
    severity = 4,
    themes = { "core", "modern" },
    summary = "old-style type tag; use the type instead",
    description = [[
        Tags such as `Float:`, `String:` and `Handle:` are the type system of old Pawn. Since
        SourcePawn 1.7 a declaration names its type in front: `float speed`, `char name[64]`,
        `Handle timer`, `Action OnTimer(Handle timer)`. Casts with tags are reported by
        Modern::ProhibitTagCasts instead.
    ]],
    on = {
        old_type = function(node, ctx)
            if node:parent():kind() == "old_type_cast" then
                return
            end
            local tag = tags.name(node:text())
            local replacement = replacements[tag] or tag
            ctx:report(node, ("old-style tag `%s:`; use the type `%s`"):format(tag, replacement))
        end,
    },
}
