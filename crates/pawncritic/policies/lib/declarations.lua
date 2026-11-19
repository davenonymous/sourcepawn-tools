-- Copyright 2026 davenonymous

-- Where the local variables of a file are declared and used, and where a
-- declaration may move to: the analysis of Variables::RequireLateDeclarations
-- and Variables::RequireNarrowDeclarations.

local constant_kinds = pawn.set({
    "int_literal", "float_literal", "char_literal", "string_literal", "packed_string_literal", "bool_literal", "null",
})

local loop_kinds = pawn.set({ "for_statement", "while_statement", "do_while_statement" })

local function is_constant(node)
    if constant_kinds[node:kind()] then
        return true
    end
    if node:kind() == "unary_expression" then
        local argument = node:child_by_field("argument")
        return argument ~= nil and is_constant(argument)
    end
    if node:kind() ~= "array_literal" then
        return false
    end
    for _, item in ipairs(node:named_children()) do
        if item:kind() ~= "comment" and not is_constant(item) then
            return false
        end
    end
    return true
end

-- Moving a declaration with no initializer or a constant one never changes what the code does.
local function has_constant_initializer(declaration)
    local value = declaration:child_by_field("initialValue")
    return value == nil or is_constant(value)
end

local function is_plain_declaration(statement)
    if statement:kind() ~= "variable_declaration_statement" then
        return false
    end
    for _, declaration in ipairs(statement:find("variable_declaration")) do
        if not has_constant_initializer(declaration) then
            return false
        end
    end
    return true
end

local function is_statement(node)
    return node:is_named() and node:kind() ~= "comment" and not pawn.starts_with(node:kind(), "preproc_")
end

local function is_array(declaration)
    for _, child in ipairs(declaration:named_children()) do
        if child:kind() == "fixed_dimension" or child:kind() == "dimension" then
            return true
        end
    end
    return false
end

-- Whether an identifier names a variable, not a member, a scope or a named argument.
local function is_variable_reference(identifier)
    local parent = identifier:parent()
    local field = identifier:field()
    if parent == nil then
        return false
    end
    if (parent:kind() == "field_access" or parent:kind() == "scope_access") and field == "field" then
        return false
    end
    if parent:kind() == "scope_access" and field == "scope" then
        return false
    end
    return not (parent:kind() == "named_arg" and field == "arg_name")
end

-- Every identifier of the file that names a variable, by its name, in document order.
local function references_by_name(file)
    local references = {}
    for _, identifier in ipairs(file:find("identifier")) do
        if is_variable_reference(identifier) then
            local name = identifier:text()
            references[name] = references[name] or {}
            table.insert(references[name], identifier)
        end
    end
    return references
end

-- The statement directly inside `block` that holds `node`.
local function statement_in(block, node)
    local current = node
    for ancestor in node:ancestors() do
        if ancestor == block then
            return current
        end
        current = ancestor
    end
    return nil
end

local function is_shadowed(block, declaration, name)
    for _, other in ipairs(block:find("variable_declaration", "old_variable_declaration")) do
        local other_name = other:child_by_field("name")
        if other ~= declaration and other_name and other_name:text() == name then
            return true
        end
    end
    return false
end

local function mentions(node, name)
    if node:kind() == "identifier" then
        return node:text() == name and is_variable_reference(node)
    end
    for _, identifier in ipairs(node:find("identifier")) do
        if identifier:text() == name and is_variable_reference(identifier) then
            return true
        end
    end
    return false
end

-- How many statements between two statements of a block do something other than declare.
local function count_gap(from, to)
    local gap = 0
    local current = from:next_named_sibling()
    while current and current ~= to do
        if is_statement(current) and not is_plain_declaration(current) then
            gap = gap + 1
        end
        current = current:next_named_sibling()
    end
    return gap
end

-- A braced body of a condition or a loop, where a declaration may move to.
local function is_target_block(node)
    if node:kind() ~= "block" then
        return false
    end
    local parent = node:parent()
    local field = node:field()
    if parent:kind() == "condition_statement" then
        return field == "truePath" or field == "falsePath"
    end
    return loop_kinds[parent:kind()] ~= nil and field == "body"
end

local function is_plain_assignment_target(use, name)
    local assignment = use:parent()
    if assignment:kind() ~= "assignment_expression" or use:field() ~= "left" then
        return false
    end
    local operator = assignment:child_by_field("operator")
    local right = assignment:child_by_field("right")
    return operator ~= nil and operator:text() == "=" and right ~= nil and not mentions(right, name)
end

-- Whether the array is handed to a call as the buffer the call fills: SourcePawn passes a buffer
-- with its size right after it, `sizeof(name)`. A call that also gets the array among its other
-- arguments reads it too, as `Format(name, sizeof(name), "%s", name)` does, and a call without
-- the size, such as `strcmp(previous, name)`, only reads it.
local function is_filled_buffer(use, name)
    local arguments = use:parent()
    if arguments:kind() ~= "call_arguments" then
        return false
    end
    local size = use:next_named_sibling()
    local sized = size and size:kind() == "sizeof_expression" and size:child_by_field("type")
    if not sized or sized:text() ~= name then
        return false
    end
    for _, argument in ipairs(arguments:named_children()) do
        if argument ~= use and argument ~= size and mentions(argument, name) then
            return false
        end
    end
    return true
end

-- Whether every pass through `target` gives the variable a new value before reading it, so no
-- value has to survive from an earlier pass of a loop: the first use is a plain assignment or
-- an array handed to a call as the buffer it fills, in a statement that always runs.
local function is_fresh_each_pass(target, first_use, declaration, name)
    local statement = statement_in(target, first_use)
    if statement:kind() == "condition_statement" then
        local condition = statement:child_by_field("condition")
        if not (condition and condition:contains(first_use)) then
            return false
        end
    elseif statement:kind() ~= "expression_statement" then
        return false
    end
    if is_plain_assignment_target(first_use, name) then
        return true
    end
    return is_array(declaration) and is_filled_buffer(first_use, name)
end

-- The node below `block` that holds every use, and the nodes from `block` down to it.
local function path_to_common_ancestor(block, uses)
    local path = {}
    local current = uses[1]
    while current and current ~= block do
        table.insert(path, 1, current)
        current = current:parent()
    end
    for depth = #path, 1, -1 do
        local holds_all = true
        for _, use in ipairs(uses) do
            if not (path[depth] == use or path[depth]:contains(use)) then
                holds_all = false
                break
            end
        end
        if holds_all then
            return { table.unpack(path, 1, depth) }
        end
    end
    return {}
end

-- The innermost block of a condition or loop that holds every use and may take the declaration.
local function narrowest_block(block, uses, declaration, name)
    local target = nil
    local inside_loop = false
    for _, node in ipairs(path_to_common_ancestor(block, uses)) do
        if is_target_block(node) and (not inside_loop or is_fresh_each_pass(node, uses[1], declaration, name)) then
            target = node
        end
        if loop_kinds[node:kind()] then
            inside_loop = true
        end
    end
    return target
end

-- Where a local variable is declared and used, or nil when its declaration cannot move safely
-- or it is never used (spcomp reports that).
local function analyze(declaration, references)
    local statement = declaration:parent()
    local block = statement and statement:parent()
    if statement:kind() ~= "variable_declaration_statement" or block == nil or block:kind() ~= "block" then
        return nil
    end
    local name_node = declaration:child_by_field("name")
    if name_node == nil or block:has_error() or not has_constant_initializer(declaration) then
        return nil
    end
    local name = name_node:text()
    local _, declaration_end = declaration:byte_range()
    local uses = {}
    for _, identifier in ipairs(references[name] or {}) do
        if identifier:byte_range() >= declaration_end and block:contains(identifier) then
            table.insert(uses, identifier)
        end
    end
    if #uses == 0 or is_shadowed(block, declaration, name) then
        return nil
    end
    local first_statement = statement_in(block, uses[1])
    if first_statement == statement then
        return nil
    end
    local target = narrowest_block(block, uses, declaration, name)
    return {
        name = name,
        first_statement = first_statement,
        gap = count_gap(statement, first_statement),
        target = target,
        target_statement = target and statement_in(target, uses[1]),
    }
end

return {
    references_by_name = references_by_name,
    analyze = analyze,
}
