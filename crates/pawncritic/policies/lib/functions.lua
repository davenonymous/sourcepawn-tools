-- Copyright 2026 davenonymous

-- Functions in the syntax tree: every kind of node that is a function with
-- a body of its own, and whether one is public.

-- Functions, enum struct and methodmap methods, constructors, destructors,
-- property accessors and operator overloads.
local kinds = {
    "function_definition", "enum_struct_method", "methodmap_method", "methodmap_method_constructor",
    "methodmap_method_destructor", "methodmap_property_method", "alias_declaration",
}

-- Whether a function is declared `public`, which the game or another plugin calls by its name.
local function is_public(function_node)
    local visibility = function_node:child_by_field("visibility")
    return visibility ~= nil and visibility:text():find("public", 1, true) ~= nil
end

return {
    kinds = kinds,
    kind_set = pawn.set(kinds),
    is_public = is_public,
}
