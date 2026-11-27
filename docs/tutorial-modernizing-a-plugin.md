# Tutorial: modernizing a plugin

This tutorial takes a realistic old plugin from start to finish: a greeter that welcomes players and
counts their visits in a database. It was written in the SourcePawn of 2013, compiles with warnings,
and has a security hole and two bugs that nobody noticed. With pawncritic and pawntidy you find
them, fix them in a sensible order, and set the project up so they do not come back.

You will learn how to:

- get an overview of a plugin with many violations,
- work through them by severity, the most important first,
- leave the layout to pawntidy,
- keep the plugin clean with a configuration file, a Git hook and CI.

It assumes both tools are installed (see [Installation](../BUILD.md#installation)) and that you
work in the plugin's repository, `/home/user/greeter`.

- [The plugin](#the-plugin)
- [Step 1: get an overview](#step-1-get-an-overview)
- [Step 2: fix what is dangerous](#step-2-fix-what-is-dangerous)
- [Step 3: fix the bugs](#step-3-fix-the-bugs)
- [Step 4: modernize the code](#step-4-modernize-the-code)
- [Step 5: let pawntidy do the layout](#step-5-let-pawntidy-do-the-layout)
- [Step 6: keep it clean](#step-6-keep-it-clean)
- [What to read next](#what-to-read-next)

## The plugin

`scripting/greeter.sp` greets every player five seconds after they join, with a message from the
ConVar `sm_greeter_message`. The command `sm_visits <name>` looks up how often a player has visited:

```sourcepawn
#include <sourcemod>

new Handle:g_hDatabase = INVALID_HANDLE;
new Handle:g_hCvarMessage;

public Plugin:myinfo =
{
	name = "Greeter",
	author = "The Server Team",
	description = "Greets players and counts their visits",
	version = "1.0",
	url = ""
};

public OnPluginStart()
{
	g_hCvarMessage = CreateConVar("sm_greeter_message", "Welcome back", "The greeting");
	RegConsoleCmd("sm_visits", Command_Visits);
	SQL_TConnect(OnDatabaseConnected, "greeter");
}

public OnDatabaseConnected(Handle:owner, Handle:hndl, const String:error[], any:data)
{
	g_hDatabase = hndl;
}

public OnClientPostAdminCheck(client)
{
	CreateTimer(5.0, Timer_Greet, client);
}

public Action:Timer_Greet(Handle:timer, any:client)
{
	decl String:message[64];
	GetConVarString(g_hCvarMessage, message, sizeof(message));
	PrintToChat(client, "%s, %N!", message, client);
}

public Action:Command_Visits(client, args)
{
	decl String:name[64];
	decl String:query[256];
	GetCmdArg(1, name, sizeof(name));
	Format(query, sizeof(query), "SELECT visits FROM greeter WHERE name = '%s'", name);
	SQL_TQuery(g_hDatabase, OnVisitsLoaded, query, client);
	return Plugin_Handled;
}

public OnVisitsLoaded(Handle:owner, Handle:hndl, const String:error[], any:client)
{
	if (hndl != INVALID_HANDLE && SQL_FetchRow(hndl))
		PrintToChat(client, "%d visits", SQL_FetchInt(hndl, 0));
}

public OnPluginEnd()
{
	for (new i = 1; i < MaxClients; i++)
		if (IsClientInGame(i)) PrintToChat(i, "Goodbye!");
	CloseHandle(g_hDatabase);
}
```

It works, most of the time. Can you spot its problems before pawncritic does?

## Step 1: get an overview

A first run on an old plugin reports a lot. Instead of reading every violation, look at the counts
first. `--statistics` adds the number of violations per policy, `--no-snippets` keeps the list
short, and the output is shortened here to the statistics:

```
$ pawncritic --statistics --no-snippets scripting/greeter.sp
...
Violations per policy:
      1  [5] Security::ProhibitSqlInjection
      1  [4] Bugs::RequireInclusiveMaxClients
      1  [4] Modern::ProhibitCloseHandle
     11  [4] Modern::ProhibitOldDeclarations
     17  [4] Modern::ProhibitOldTypeTags
      1  [4] Modern::RequireNewDecls
      1  [4] Modern::RequireSemicolons
      1  [3] Bugs::RequireUserIdInTimers
      3  [3] CodeLayout::RequireBracedBlocks
      7  [3] CodeLayout::RequireSameLineBraces
      4  [3] Modern::ProhibitLegacyApiFunctions
      1  [3] Philosophy::ProhibitBareHandle

49 violations in 1 of 1 file (1 of severity 5, 32 of severity 4, 16 of severity 3).
```

49 violations sound like a lot, but they fall into four groups:

| Group | Policies | Work |
|---|---|---|
| Dangerous | `Security::ProhibitSqlInjection` | One line, and urgent. |
| Bugs | `Bugs::RequireInclusiveMaxClients`, `Bugs::RequireUserIdInTimers` | Two small fixes. |
| Old syntax and API | the `Modern::` policies, `Philosophy::ProhibitBareHandle` | A rewrite of the declarations, mostly mechanical. |
| Layout | the `CodeLayout::` policies | Nothing: pawntidy does it. |

The severity in brackets is the order to work in: 5 first. `pawncritic --explain <name>` explains
any of them.

## Step 2: fix what is dangerous

`--gentle` reports only severity 5:

```
$ pawncritic --gentle scripting/greeter.sp
scripting/greeter.sp:44:79: [5] `name` holds client input and goes into a query unescaped  Security::ProhibitSqlInjection
   44 |     Format(query, sizeof(query), "SELECT visits FROM greeter WHERE name = '%s'", name);
      |                                                                                  ^^^^
      = hint: format the query with Database.Format()
```

The argument of `sm_visits` goes into the query as it is. Any player can type
`sm_visits "x' OR '1'='1"` and change what the query does. Fix it now, before anything else, so that
the fix can ship on its own. The smallest fix escapes the name with `SQL_EscapeString()`, which
pawncritic knows as a sanitizer:

```sourcepawn
public Action:Command_Visits(client, args)
{
	decl String:name[64];
	decl String:safe[129];
	decl String:query[256];
	GetCmdArg(1, name, sizeof(name));
	SQL_EscapeString(g_hDatabase, name, safe, sizeof(safe));
	Format(query, sizeof(query), "SELECT visits FROM greeter WHERE name = '%s'", safe);
	SQL_TQuery(g_hDatabase, OnVisitsLoaded, query, client);
	return Plugin_Handled;
}
```

```
$ pawncritic --gentle scripting/greeter.sp
No violations in 1 file.
```

The modern way, `Database.Format()`, follows in [Step 4](#step-4-modernize-the-code).

## Step 3: fix the bugs

Bugs come before style. `--theme bugs` runs only the policies about likely bugs, and `--stern` those
of severity 4 and 5:

```
$ pawncritic --stern --theme bugs scripting/greeter.sp
scripting/greeter.sp:59:18: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
   59 |     for (new i = 1; i < MaxClients; i++)
      |                     ^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`
```

On a full server, the last player never gets the goodbye. The fix is `i <= MaxClients`.

The default severity, 3, shows one more bug. `OnClientPostAdminCheck` passes the client index to a
timer:

```sourcepawn
CreateTimer(5.0, Timer_Greet, client);
```

`Bugs::RequireUserIdInTimers` reports it. If the player leaves within the five seconds,
`PrintToChat()` fails with an error, because the client is not in game. If another player has taken
the slot by then, the newcomer gets a greeting that was meant for someone else.

Pass the user id instead, and convert it back in the callback; `GetClientOfUserId()` gives 0 when
the player is gone. The same applies to the `client` passed to `SQL_TQuery()`: pawncritic checks
only timers and frame callbacks, but a database answer arrives just as late. Both fixes are part of
the next step.

## Step 4: modernize the code

The `Modern::` policies make up most of the list. SourcePawn 1.7 replaced tags with types and handle
functions with methodmaps, and the compiler checks the new code far better. The conversion is
mechanical:

| Old | New | Policy |
|---|---|---|
| `new Handle:g_hDatabase` | `Database g_hDatabase` | `Modern::ProhibitOldDeclarations`, `Modern::ProhibitOldTypeTags` |
| `new Handle:g_hCvarMessage` | `ConVar g_hCvarMessage` | `Philosophy::ProhibitBareHandle` |
| `public OnPluginStart()` | `public void OnPluginStart()` | `Modern::ProhibitOldDeclarations` |
| `decl String:message[64]` | `char message[64]` | `Modern::ProhibitOldDeclarations` |
| `GetConVarString(g_hCvarMessage, ...)` | `g_hCvarMessage.GetString(...)` | `Modern::ProhibitLegacyApiFunctions` |
| `SQL_TQuery`, `SQL_FetchRow`, `SQL_FetchInt` | `Database.Query`, `DBResultSet.FetchRow`, `DBResultSet.FetchInt` | `Modern::ProhibitLegacyApiFunctions` |
| `SQL_TConnect` | `Database.Connect` | none; it belongs to the same conversion |
| `CloseHandle(g_hDatabase)` | `delete g_hDatabase` | `Modern::ProhibitCloseHandle` |
| no pragmas | `#pragma semicolon 1` and `#pragma newdecls required` | `Modern::RequireSemicolons`, `Modern::RequireNewDecls` |

`#pragma newdecls required` makes the compiler refuse old declarations from now on, so the plugin
cannot slide back. `Database.Format()` escapes its `%s` arguments, so the extra buffer of Step 2
goes away. Here is the plugin after the conversion, with the two bug fixes of Step 3. The layout is
still the old one:

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

Database g_hDatabase;
ConVar g_hCvarMessage;

public Plugin myinfo =
{
	name = "Greeter",
	author = "The Server Team",
	description = "Greets players and counts their visits",
	version = "1.1",
	url = ""
};

public void OnPluginStart()
{
	g_hCvarMessage = CreateConVar("sm_greeter_message", "Welcome back", "The greeting");
	RegConsoleCmd("sm_visits", Command_Visits);
	Database.Connect(OnDatabaseConnected, "greeter");
}

public void OnDatabaseConnected(Database db, const char[] error, any data)
{
	g_hDatabase = db;
}

public void OnClientPostAdminCheck(int client)
{
	CreateTimer(5.0, Timer_Greet, GetClientUserId(client));
}

public Action Timer_Greet(Handle timer, any userid)
{
	int client = GetClientOfUserId(userid);
	if (client == 0)
		return Plugin_Stop;
	char message[64];
	g_hCvarMessage.GetString(message, sizeof(message));
	PrintToChat(client, "%s, %N!", message, client);
	return Plugin_Stop;
}

public Action Command_Visits(int client, int args)
{
	char name[64];
	GetCmdArg(1, name, sizeof(name));
	char query[256];
	g_hDatabase.Format(query, sizeof(query), "SELECT visits FROM greeter WHERE name = '%s'", name);
	g_hDatabase.Query(OnVisitsLoaded, query, GetClientUserId(client));
	return Plugin_Handled;
}

public void OnVisitsLoaded(Database db, DBResultSet results, const char[] error, any userid)
{
	int client = GetClientOfUserId(userid);
	if (client != 0 && results != null && results.FetchRow())
		PrintToChat(client, "%d visits", results.FetchInt(0));
}

public void OnPluginEnd()
{
	for (int i = 1; i <= MaxClients; i++)
		if (IsClientInGame(i)) PrintToChat(i, "Goodbye!");
	delete g_hDatabase;
}
```

Severity 4 and 5 are clean now:

```
$ pawncritic --stern scripting/greeter.sp
No violations in 1 file.
```

Compile the plugin and test it on a server before you go on. pawncritic reads the code without
compiling it, so it does not replace the compiler.

## Step 5: let pawntidy do the layout

What is left at the default severity is layout: braces on lines of their own, and bodies without
braces.

```
$ pawncritic --statistics --no-snippets scripting/greeter.sp
...
Violations per policy:
      4  [3] CodeLayout::RequireBracedBlocks
      7  [3] CodeLayout::RequireSameLineBraces

11 violations in 1 of 1 file (11 of severity 3).
```

Nobody should fix these by hand. `pawntidy --diff` shows what pawntidy would change (the output is
shortened):

```
$ pawntidy --diff scripting/greeter.sp
--- a/scripting/greeter.sp
+++ b/scripting/greeter.sp
@@ -6,8 +6,7 @@
 Database g_hDatabase;
 ConVar g_hCvarMessage;
 
-public Plugin myinfo =
-{
+public Plugin myinfo = {
 	name = "Greeter",
 	author = "The Server Team",
 	description = "Greets players and counts their visits",
...
```

`pawntidy --write` rewrites the file. It checks its result token by token against the input before
it writes, so the code itself cannot change:

```
$ pawntidy --write scripting/greeter.sp
reformatted scripting/greeter.sp
1 file reformatted, 0 files left unchanged.
```

The bodies now have braces, and every opening brace stands on the line of its statement:

```sourcepawn
public Action Timer_Greet(Handle timer, any userid) {
	int client = GetClientOfUserId(userid);
	if (client == 0) {
		return Plugin_Stop;
	}
	char message[64];
	g_hCvarMessage.GetString(message, sizeof(message));
	PrintToChat(client, "%s, %N!", message, client);
	return Plugin_Stop;
}
```

The plugin is clean, even at the strictest severity:

```
$ pawncritic --brutal scripting/greeter.sp
No violations in 1 file.
```

## Step 6: keep it clean

A clean plugin stays clean only if every change is checked. Three pieces do that.

**A configuration file.** `.pawncritic.toml` at the root of the repository makes every run check the
same way, from the command line, the editor, the hook and CI. This project keeps the default
severity and leaves the layout to pawntidy:

```toml
theme = "!tidy"
```

**A Git hook.** The [pre-commit hook](editors-and-ci.md#git-pre-commit-hook) refuses a commit whose
files are not formatted or have a violation of severity 4 or 5. It catches most problems before they
leave your machine.

**CI.** The [GitHub Action](github-action.md) checks every push and pull request. Save this as
`.github/workflows/lint.yml`:

```yaml
name: lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: davenonymous/sourcepawn-tools@v0
        with:
          paths: scripting
          tidy: true
```

A pull request that brings back an old declaration, or a `CloseHandle()`, now gets a GitHub
annotation on its line and a failed check.

## What to read next

- [Vetting a third-party plugin](vetting-plugins.md): use the security policies on a plugin you did
  not write.
- [Built-in policies](policies.md): every policy with an example.
- [pawncritic](pawncritic.md#the-configuration-file): the configuration file in detail.
- [Editors and CI](editors-and-ci.md): see violations while you type.
- [Writing policies](writing-policies.md): check your project's own conventions.
