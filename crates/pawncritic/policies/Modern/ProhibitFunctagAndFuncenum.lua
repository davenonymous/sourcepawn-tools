-- Copyright 2026 davenonymous

return {
    name = "Modern::ProhibitFunctagAndFuncenum",
    severity = 4,
    themes = { "core", "modern" },
    summary = "`functag` and `funcenum` are obsolete; use `typedef` and `typeset`",
    description = [[
        `functag` and `funcenum` declare function types in old Pawn syntax. Since SourcePawn 1.7,
        `typedef MyCallback = function void (int client);` declares one signature and
        `typeset MyCallbacks { function void (int client); function void (); };` several.
    ]],
    query = "[(functag) (funcenum)] @violation",
}
