-- Copyright 2026 davenonymous

-- Functions of the old handle API and the methodmap members that replace
-- them, from SourceMod 1.7 on.
local replacements = {
    -- ArrayList
    CreateArray = "new ArrayList(...)",
    ClearArray = "ArrayList.Clear()",
    CloneArray = "ArrayList.Clone()",
    ResizeArray = "ArrayList.Resize()",
    GetArraySize = "ArrayList.Length",
    PushArrayCell = "ArrayList.Push()",
    PushArrayString = "ArrayList.PushString()",
    PushArrayArray = "ArrayList.PushArray()",
    GetArrayCell = "ArrayList.Get()",
    GetArrayString = "ArrayList.GetString()",
    GetArrayArray = "ArrayList.GetArray()",
    SetArrayCell = "ArrayList.Set()",
    SetArrayString = "ArrayList.SetString()",
    SetArrayArray = "ArrayList.SetArray()",
    ShiftArrayUp = "ArrayList.ShiftUp()",
    RemoveFromArray = "ArrayList.Erase()",
    SwapArrayItems = "ArrayList.SwapAt()",
    FindStringInArray = "ArrayList.FindString()",
    FindValueInArray = "ArrayList.FindValue()",
    SortADTArray = "ArrayList.Sort()",
    -- ArrayStack
    CreateStack = "new ArrayStack(...)",
    PushStackCell = "ArrayStack.Push()",
    PushStackString = "ArrayStack.PushString()",
    PopStackCell = "ArrayStack.Pop()",
    PopStackString = "ArrayStack.PopString()",
    IsStackEmpty = "ArrayStack.Empty",
    -- StringMap
    CreateTrie = "new StringMap()",
    SetTrieValue = "StringMap.SetValue()",
    SetTrieString = "StringMap.SetString()",
    SetTrieArray = "StringMap.SetArray()",
    GetTrieValue = "StringMap.GetValue()",
    GetTrieString = "StringMap.GetString()",
    GetTrieArray = "StringMap.GetArray()",
    RemoveFromTrie = "StringMap.Remove()",
    ClearTrie = "StringMap.Clear()",
    GetTrieSize = "StringMap.Size",
    CreateTrieSnapshot = "StringMap.Snapshot()",
    -- DataPack
    CreateDataPack = "new DataPack()",
    WritePackCell = "DataPack.WriteCell()",
    WritePackFloat = "DataPack.WriteFloat()",
    WritePackString = "DataPack.WriteString()",
    WritePackFunction = "DataPack.WriteFunction()",
    ReadPackCell = "DataPack.ReadCell()",
    ReadPackFloat = "DataPack.ReadFloat()",
    ReadPackString = "DataPack.ReadString()",
    ReadPackFunction = "DataPack.ReadFunction()",
    ResetPack = "DataPack.Reset()",
    GetPackPosition = "DataPack.Position",
    SetPackPosition = "DataPack.Position",
    IsPackReadable = "DataPack.IsReadable()",
    -- ConVar
    GetConVarInt = "ConVar.IntValue",
    GetConVarFloat = "ConVar.FloatValue",
    GetConVarBool = "ConVar.BoolValue",
    GetConVarString = "ConVar.GetString()",
    SetConVarInt = "ConVar.IntValue",
    SetConVarFloat = "ConVar.FloatValue",
    SetConVarBool = "ConVar.BoolValue",
    SetConVarString = "ConVar.SetString()",
    ResetConVar = "ConVar.RestoreDefault()",
    HookConVarChange = "ConVar.AddChangeHook()",
    UnhookConVarChange = "ConVar.RemoveChangeHook()",
    GetConVarFlags = "ConVar.Flags",
    SetConVarFlags = "ConVar.Flags",
    GetConVarName = "ConVar.GetName()",
    GetConVarBounds = "ConVar.GetBounds()",
    SetConVarBounds = "ConVar.SetBounds()",
    GetConVarDefault = "ConVar.GetDefault()",
    -- Menu and Panel
    CreateMenu = "new Menu(...)",
    DisplayMenu = "Menu.Display()",
    DisplayMenuAtItem = "Menu.DisplayAt()",
    AddMenuItem = "Menu.AddItem()",
    InsertMenuItem = "Menu.InsertItem()",
    RemoveMenuItem = "Menu.RemoveItem()",
    RemoveAllMenuItems = "Menu.RemoveAllItems()",
    GetMenuItem = "Menu.GetItem()",
    GetMenuItemCount = "Menu.ItemCount",
    SetMenuTitle = "Menu.SetTitle()",
    GetMenuTitle = "Menu.GetTitle()",
    SetMenuExitButton = "Menu.ExitButton",
    SetMenuExitBackButton = "Menu.ExitBackButton",
    SetMenuPagination = "Menu.Pagination",
    SetMenuOptionFlags = "Menu.OptionFlags",
    CancelMenu = "Menu.Cancel()",
    CreatePanel = "new Panel()",
    SetPanelTitle = "Panel.SetTitle()",
    DrawPanelItem = "Panel.DrawItem()",
    DrawPanelText = "Panel.DrawText()",
    SendPanelToClient = "Panel.Send()",
    -- KeyValues
    CreateKeyValues = "new KeyValues(...)",
    FileToKeyValues = "KeyValues.ImportFromFile()",
    KeyValuesToFile = "KeyValues.ExportToFile()",
    KvJumpToKey = "KeyValues.JumpToKey()",
    KvGotoFirstSubKey = "KeyValues.GotoFirstSubKey()",
    KvGotoNextKey = "KeyValues.GotoNextKey()",
    KvGoBack = "KeyValues.GoBack()",
    KvRewind = "KeyValues.Rewind()",
    KvGetSectionName = "KeyValues.GetSectionName()",
    KvSetSectionName = "KeyValues.SetSectionName()",
    KvGetString = "KeyValues.GetString()",
    KvGetNum = "KeyValues.GetNum()",
    KvGetFloat = "KeyValues.GetFloat()",
    KvGetColor = "KeyValues.GetColor()",
    KvGetVector = "KeyValues.GetVector()",
    KvSetString = "KeyValues.SetString()",
    KvSetNum = "KeyValues.SetNum()",
    KvSetFloat = "KeyValues.SetFloat()",
    KvDeleteKey = "KeyValues.DeleteKey()",
    KvDeleteThis = "KeyValues.DeleteThis()",
    -- Files
    ReadFileLine = "File.ReadLine()",
    WriteFileLine = "File.WriteLine()",
    IsEndOfFile = "File.EndOfFile()",
    FileSeek = "File.Seek()",
    FilePosition = "File.Position",
    FlushFile = "File.Flush()",
    ReadDirEntry = "DirectoryListing.GetNext()",
    -- Databases
    SQL_TQuery = "Database.Query()",
    SQL_FetchRow = "DBResultSet.FetchRow()",
    SQL_FetchInt = "DBResultSet.FetchInt()",
    SQL_FetchFloat = "DBResultSet.FetchFloat()",
    SQL_FetchString = "DBResultSet.FetchString()",
    SQL_GetRowCount = "DBResultSet.RowCount",
    SQL_GetAffectedRows = "Database.AffectedRows",
    SQL_GetInsertId = "Database.InsertId",
    SQL_EscapeString = "Database.Escape()",
    SQL_SetCharset = "Database.SetCharset()",
    -- Forwards
    CreateGlobalForward = "new GlobalForward(...)",
    CreateForward = "new PrivateForward(...)",
    -- Events
    GetEventInt = "Event.GetInt()",
    GetEventFloat = "Event.GetFloat()",
    GetEventBool = "Event.GetBool()",
    GetEventString = "Event.GetString()",
    SetEventInt = "Event.SetInt()",
    SetEventFloat = "Event.SetFloat()",
    SetEventBool = "Event.SetBool()",
    SetEventString = "Event.SetString()",
    FireEvent = "Event.Fire()",
    CancelCreatedEvent = "Event.Cancel()",
}

-- The options read once: the options of a policy's state never change,
-- so they are kept for as long as ctx.options is the same table. `extra`
-- maps names to replacements, given as "Name=Replacement".
local parsed_options, allowed, extra_replacements
local function read_options(options)
    if parsed_options ~= options then
        parsed_options = options
        allowed = pawn.set(options.allow)
        extra_replacements = {}
        for _, entry in ipairs(options.extra) do
            local name, replacement = entry:match("^%s*([%w_]+)%s*=(.*)$")
            if name then
                extra_replacements[name] = pawn.trim(replacement)
            end
        end
    end
    return allowed, extra_replacements
end

return {
    name = "Modern::ProhibitLegacyApiFunctions",
    severity = 3,
    themes = { "core", "modern" },
    summary = "a function of the old handle API; use its methodmap member",
    description = [[
        Since SourceMod 1.7 handles have methodmaps: `ArrayList`, `StringMap`, `DataPack`, `ConVar`,
        `Menu`, `KeyValues`, `File`, `Database`, `Event` and others. Their members say what they
        work on, `list.Push(value)` instead of `PushArrayCell(list, value)`, and make the compiler
        check the handle's type. The option `allow` lists functions not to report; `extra` adds
        functions as `"Name=Replacement"`.
    ]],
    options = {
        allow = { default = {}, description = "Functions not to report." },
        extra = { default = {}, description = "More functions to report, as \"Name=Replacement\"." },
    },
    on = {
        call_expression = function(node, ctx)
            local callee = node:child_by_field("function")
            if callee:kind() ~= "identifier" then
                return
            end
            local name = callee:text()
            local allowed_names, extra = read_options(ctx.options)
            if allowed_names[name] then
                return
            end
            local replacement = replacements[name] or extra[name]
            if replacement then
                ctx:report(callee, ("`%s()` is the old handle API; use `%s`"):format(name, replacement))
            end
        end,
    },
}
