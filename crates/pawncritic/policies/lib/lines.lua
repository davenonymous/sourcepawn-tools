-- Copyright 2026 davenonymous

-- The lines of a file that hold code.

-- The lines that hold code, as a set of line numbers, and how many there are: every line a
-- token other than a comment touches.
local function code_lines(file)
    local lines = {}
    local count = 0
    for _, token in ipairs(file:tokens()) do
        if token.kind ~= "line_comment" and token.kind ~= "block_comment" then
            for line = token.line, token.end_line do
                if not lines[line] then
                    lines[line] = true
                    count = count + 1
                end
            end
        end
    end
    return lines, count
end

return {
    code_lines = code_lines,
}
