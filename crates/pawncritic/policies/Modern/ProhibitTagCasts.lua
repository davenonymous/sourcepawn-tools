-- Copyright 2026 davenonymous

local tags = pawn.library("tags")

local replacements = {
    Float = "float",
    _ = "int",
    bool = "bool",
}

return {
    name = "Modern::ProhibitTagCasts",
    severity = 4,
    themes = { "core", "modern" },
    summary = "old-style tag cast; use `view_as<Type>(value)`",
    description = [[
        A tag cast like `_:value` or `Float:value` reinterprets a value without telling the reader
        what happens. `view_as<int>(value)` says the same with the type spelled out and is the form
        the compiler documents.
    ]],
    on = {
        old_type_cast = function(node, ctx)
            local tag_node = node:child_by_field("type")
            local value_node = node:child_by_field("value")
            local tag = tags.name(tag_node:text())
            local type_name = replacements[tag] or tag
            ctx:report(node, ("old-style tag cast `%s:`; use `view_as<%s>(...)`"):format(tag, type_name), {
                hint = ("view_as<%s>(%s)"):format(type_name, value_node:text()),
            })
        end,
    },
}
