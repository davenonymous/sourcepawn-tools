# Built-in policies

Every policy that ships inside pawncritic: what it reports, why it matters, an example of code it
reports and the fix, and its options. Each policy has its own heading, so a search for its name,
such as `ProhibitCloseHandle`, lands on its entry.

The table of all policies with their severities and themes is in
[pawncritic.md](pawncritic.md#built-in-policies). How to choose which policies run is in [Choosing
policies](pawncritic.md#choosing-policies), and how to set an option in [Policy
sections](pawncritic.md#policy-sections). `pawncritic --explain <name>` prints the same explanation
on the command line.

- [Reading an entry](#reading-an-entry)
- [Core](#core): `RequireParsableCode`
- [Bugs](#bugs): `RequireInclusiveMaxClients`, `RequireUserIdInTimers`
- [CodeLayout](#codelayout): `ProhibitLongLines`, `ProhibitMixedIndentation`,
  `ProhibitTrailingWhitespace`, `RequireBracedBlocks`, `RequireSameLineBraces`, `RequireTidyCode`
- [Complexity](#complexity): `ProhibitLongFiles`, `ProhibitLongFunctions`
- [Malicious](#malicious): `ProhibitClientSlowhack`, `ProhibitDangerousDownloads`,
  `ProhibitObfuscatedStrings`, `ProhibitPasswordConVars`, `ProhibitSteamIdBackdoors`,
  `ProhibitVguiCommandBypass`
- [Modern](#modern): `ProhibitCloseHandle`, `ProhibitDeprecatedFunctions`,
  `ProhibitFunctagAndFuncenum`, `ProhibitLegacyApiFunctions`, `ProhibitOldDeclarations`,
  `ProhibitOldTypeTags`, `ProhibitTagCasts`, `RequireNewDecls`, `RequireSemicolons`
- [NamingConventions](#namingconventions): `RequireGlobalPrefix`
- [Philosophy](#philosophy): `ProhibitBareHandle`, `ProhibitDeepNesting`, `ProhibitElseAfterReturn`,
  `ProhibitEmptyBlocks`, `ProhibitHiddenGlobalMutation`, `ProhibitMeaninglessNames`,
  `RequireBooleanPredicateNames`, `RequireEarlyExit`
- [Security](#security): `ProhibitAbsolutePaths`, `ProhibitCodeFileWrites`,
  `ProhibitCommandInjection`, `ProhibitHardcodedEndpoints`, `ProhibitNonLiteralFormat`,
  `ProhibitPathTraversal`, `ProhibitSecretFileAccess`, `ProhibitServerControlCommands`,
  `ProhibitShellExecution`, `ProhibitSqlInjection`
- [Variables](#variables): `RequireLateDeclarations`, `RequireNarrowDeclarations`

## Reading an entry

Each entry starts with why the policy exists and a line with its severity, themes and options. Then
come:

- **Reported:** a short piece of code the policy reports, saved as `plugin.sp`, and the output of
  `pawncritic --single-policy <name> plugin.sp`. A tab in the code is shown as four spaces in the
  output.
- **Fixed:** the same code written so that the policy reports nothing.
- **Details:** exactly what the policy looks at, and what it leaves alone.
- **Message and hint:** the text of its violations. A placeholder such as `<name>` or `<n>` stands
  for the value of the case at hand. "The summary" is the policy's summary, the message in the table
  of [pawncritic.md](pawncritic.md#built-in-policies).
- **Options:** what a [policy section](pawncritic.md#policy-sections) can set, with the type and the
  default.

The examples are fragments: they leave out `#include <sourcemod>` and the declarations of the
globals they use. pawncritic does not need them, because it reads each file on its own (see [How
pawncritic reads code](pawncritic.md#how-pawncritic-reads-code)).

## Core

### Core::RequireParsableCode

pawncritic reads your code with a grammar instead of compiling it. Where the grammar cannot place
the code, the other policies see an error node instead of the real structure, and may miss
violations or report wrong ones. This policy tells you where that happens.

Usually such code does not compile either. Sometimes valid code is beyond the grammar, such as a
rare macro trick.

Severity 5 (gentle). Themes: `bugs`, `core`. A [native policy](#native-policies). Option:
`max_reports`.

**Reported:** a missing closing parenthesis.

```sourcepawn
public void OnPluginStart() {
	int count = GetClientCount();
	PrintToServer("players: %d", count;
}
```

```
$ pawncritic --single-policy RequireParsableCode plugin.sp
plugin.sp:3:36: [5] syntax error: missing `)`  Core::RequireParsableCode
    3 |     PrintToServer("players: %d", count;
      |                                       ^

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	int count = GetClientCount();
	PrintToServer("players: %d", count);
}
```

**Details:**

- Reports each place where the grammar cannot place the code, up to `max_reports` per file. An error
  inside another error is not reported again.
- When valid code is beyond the grammar, silence the line with `// no critic (RequireParsableCode)`
  (see [Annotations](pawncritic.md#annotations)).

**Message:** ``syntax error: missing `<token>` `` where the parser expected a token that is not
there, ``syntax error: unexpected `<text>` `` with the first word of the text it could not read (at
most 30 characters), or `syntax error` when that text is empty. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `max_reports` | integer | `5` | The most places reported per file. `0` or less reports none. |

## Bugs

### Bugs::RequireInclusiveMaxClients

Client indices run from 1 to `MaxClients`, both included. A loop with `i < MaxClients` never reaches
the last slot. The bug only shows on a full server, which makes it hard to find by testing.

Severity 4 (stern). Themes: `bugs`, `core`. No options.

**Reported:**

```sourcepawn
void AnnounceToAll(const char[] text) {
	for (int client = 1; client < MaxClients; client++) {
		if (IsClientInGame(client)) {
			PrintToChat(client, "%s", text);
		}
	}
}
```

```
$ pawncritic --single-policy RequireInclusiveMaxClients plugin.sp
plugin.sp:2:23: [4] the loop stops before the last client; client indices run from 1 to MaxClients  Bugs::RequireInclusiveMaxClients
    2 |     for (int client = 1; client < MaxClients; client++) {
      |                          ^^^^^^^^^^^^^^^^^^^
      = hint: compare with `<= MaxClients`

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
void AnnounceToAll(const char[] text) {
	for (int client = 1; client <= MaxClients; client++) {
		if (IsClientInGame(client)) {
			PrintToChat(client, "%s", text);
		}
	}
}
```

**Details:** reports the condition of a `for` loop that is `X < MaxClients`, `X < GetMaxClients()`,
`MaxClients > X` or `GetMaxClients() > X`.

**Message:** the summary. **Hint:** `` compare with `<= MaxClients` ``.

### Bugs::RequireUserIdInTimers

A timer or frame callback runs later. By then the player may have left, and another player may have
taken the same client index. The callback then acts on the wrong player.

A user id is never given to another player while the server runs. Pass `GetClientUserId(client)`,
and convert it back with `GetClientOfUserId(userid)`, which gives 0 when the player is gone. For
entities, pass `EntIndexToEntRef(entity)` and use `EntRefToEntIndex()`.

Severity 3 (harsh). Themes: `bugs`, `core`. Option: `client_names`.

**Reported:**

```sourcepawn
public void OnClientPutInServer(int client) {
	CreateTimer(10.0, Timer_Welcome, client);
}

public Action Timer_Welcome(Handle timer, int client) {
	PrintToChat(client, "Welcome to the server!");
	return Plugin_Stop;
}
```

```
$ pawncritic --single-policy RequireUserIdInTimers plugin.sp
plugin.sp:2:35: [3] `client` passes a client index to CreateTimer(); pass `GetClientUserId(client)`  Bugs::RequireUserIdInTimers
    2 |     CreateTimer(10.0, Timer_Welcome, client);
      |                                      ^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public void OnClientPutInServer(int client) {
	CreateTimer(10.0, Timer_Welcome, GetClientUserId(client));
}

public Action Timer_Welcome(Handle timer, int userid) {
	int client = GetClientOfUserId(userid);
	if (client != 0) {
		PrintToChat(client, "Welcome to the server!");
	}
	return Plugin_Stop;
}
```

**Details:** reports a call of `CreateTimer()` whose third argument, or of `RequestFrame()` whose
second argument, is a variable named in `client_names`. Only a plain variable name is checked, with
its exact case: `CreateTimer(1.0, Timer_Hit, attacker)` is reported,
`CreateTimer(1.0, Timer_Hit, GetAttacker())` is not.

**Message:** ``` `<name>` passes a client index to <function>(); pass `GetClientUserId(<name>)` ```,
where `<function>` is `CreateTimer` or `RequestFrame`. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `client_names` | list | `["client", "iClient", "target", "attacker", "victim", "player", "owner", "builder"]` | The names of variables that hold client indices. |

## CodeLayout

The layout policies report what [pawntidy](pawntidy.md) fixes, so they all carry the theme `tidy`.
Where pawntidy runs anyway, for example in CI, leave them out with `--theme '!tidy'`. pawntidy
cannot shorten every line, so `CodeLayout::ProhibitLongLines` can still report a file that pawntidy
formatted.

### CodeLayout::ProhibitLongLines

Long lines are hard to read, side by side in a diff and in a review. pawntidy wraps long lines where
the code allows a break, and this policy reports the ones that are left.

Severity 2 (cruel). Themes: `core`, `layout`, `tidy`. Options: `max_length`, `tab_width`.

**Reported:**

```sourcepawn
public void OnPluginStart() {
	g_cvEnabled = CreateConVar("sm_welcome_enabled", "1", "Whether new players get a welcome message when they join", _, true, 0.0, true, 1.0);
}
```

```
$ pawncritic --single-policy ProhibitLongLines plugin.sp
plugin.sp:2:1: [2] the line is 143 columns wide, the limit is 120  CodeLayout::ProhibitLongLines
    2 |     g_cvEnabled = CreateConVar("sm_welcome_enabled", "1", "Whether new players get a welcome message when they join", _, true, 0.0, true, 1.0);
      | ^

1 violation in 1 of 1 file (1 of severity 2).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	g_cvEnabled = CreateConVar("sm_welcome_enabled", "1", "Whether new players get a welcome message when they join",
	                           _, true, 0.0, true, 1.0);
}
```

**Details:** reports each line wider than `max_length` columns. A tab reaches to the next multiple
of `tab_width`, a wide character (such as a CJK character) counts as 2 columns, and a zero-width
character as 0.

**Message:** `the line is <n> columns wide, the limit is <max>`. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `max_length` | integer | `120` | The widest a line may be, in columns. |
| `tab_width` | integer | `4` | How many columns a tab stop is wide. `0` counts as 1. |

### CodeLayout::ProhibitMixedIndentation

Indentation that mixes tabs and spaces looks right only at the tab width of its author. In every
other editor the code is out of line. This policy checks that a file is indented one way: with tabs,
or with spaces.

Severity 4 (stern). Themes: `core`, `layout`, `tidy`. Option: `indent_style`.

**Reported:** line 3 is indented with four spaces, the others with a tab.

```sourcepawn
public void OnPluginStart() {
	RegConsoleCmd("sm_hello", Command_Hello);
    RegConsoleCmd("sm_bye", Command_Bye);
}
```

```
$ pawncritic --single-policy ProhibitMixedIndentation plugin.sp
plugin.sp:3:1: [4] mixed indentation; 1 line is not indented with tabs  CodeLayout::ProhibitMixedIndentation
    3 |     RegConsoleCmd("sm_bye", Command_Bye);
      | ^^^^
      = hint: line 3 differs; run `pawntidy --write plugin.sp` with indent_style = "tabs" in .pawntidy.toml

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** every line indented with tabs. `pawntidy --write plugin.sp` makes this change.

```sourcepawn
public void OnPluginStart() {
	RegConsoleCmd("sm_hello", Command_Hello);
	RegConsoleCmd("sm_bye", Command_Bye);
}
```

**Details:**

- **Which style the file has.** With `indent_style = "auto"`, the file decides. The lines that count
  are those whose first token starts a statement, a declaration, a member, an enum entry, a comment,
  a directive, a `}` or an `else`. The style most of them are indented with is the file's style; an
  indentation that starts with a tab counts for tabs, one that starts with a space for spaces. On a
  tie, tabs win. A file where none of these lines is indented is not checked.
- **A file indented with tabs.** A line breaks the style when its indentation has a space before a
  tab, or when it is one of the lines that count and its indentation holds a space. Spaces after the
  tabs are fine on a continuation line, where they align the line with the line above.
- **A file indented with spaces.** A line breaks the style when its indentation holds a tab.
- Lines inside a token that spans several lines, such as the inner lines of a block comment, are not
  checked.
- The policy reports once per file, at the first line that breaks the style, and lists the other
  lines in the hint. It does not check how many spaces a line has;
  [`CodeLayout::RequireTidyCode`](#codelayoutrequiretidycode) covers that.

**Message:** `mixed indentation; <n> lines are not indented with <style>` (`1 line is` for one),
where `<style>` is `tabs` or `spaces`. **Hint:**
``<lines> differ; run `pawntidy --write <path>` with indent_style = "<style>" in .pawntidy.toml``.
For one line, the hint says `line <n> differs`. `<lines>` lists the lines that break the style:
`line 9`, `lines 9 and 14`, `lines 9, 14 and 20`; after ten lines it ends in `and <n> more`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `indent_style` | string | `"auto"` | `"auto"` takes the style most lines use. `"tabs"` and `"spaces"` require that style. Any other value makes the policy fail on every file with ``the option indent_style is "<value>", but must be "auto", "tabs" or "spaces"``. |

### CodeLayout::ProhibitTrailingWhitespace

Whitespace at the end of a line is invisible. Every editor that strips it changes the line, and the
diff shows a change nobody made on purpose. pawntidy removes it.

Severity 2 (cruel). Themes: `core`, `layout`, `tidy`. No options.

**Reported:** line 2 ends in two spaces.

```sourcepawn
public void OnPluginStart() {
	RegConsoleCmd("sm_hello", Command_Hello);  
}
```

```
$ pawncritic --single-policy ProhibitTrailingWhitespace plugin.sp
plugin.sp:2:43: [2] the line ends in whitespace  CodeLayout::ProhibitTrailingWhitespace
    2 |     RegConsoleCmd("sm_hello", Command_Hello);  
      |                                              ^^

1 violation in 1 of 1 file (1 of severity 2).
```

**Fixed:** the same code without the two spaces.

**Details:** reports the spaces and tabs at the end of each line.

**Message:** the summary. **Hint:** none.

### CodeLayout::RequireBracedBlocks

A body without braces holds exactly one statement. A line added below it later looks like part of
the body but runs every time. With braces around every body, that mistake cannot happen, and every
body looks the same.

Severity 3 (harsh). Themes: `core`, `layout`, `tidy`. Option: `check_switch_cases`.

**Reported:**

```sourcepawn
public Action Command_Hello(int client, int args) {
	if (client == 0)
		return Plugin_Handled;
	PrintToChat(client, "Hello!");
	return Plugin_Handled;
}
```

```
$ pawncritic --single-policy RequireBracedBlocks plugin.sp
plugin.sp:3:3: [3] the body of `if` is not in braces  CodeLayout::RequireBracedBlocks
    3 |         return Plugin_Handled;
      |         ^^^^^^^^^^^^^^^^^^^^^^
      = hint: wrap it in `{ ... }`

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public Action Command_Hello(int client, int args) {
	if (client == 0) {
		return Plugin_Handled;
	}
	PrintToChat(client, "Hello!");
	return Plugin_Handled;
}
```

**Details:** reports a body of `if`, `else`, `for`, `while` or `do` that is not in braces, and with
`check_switch_cases` also one of `case` or `default`. In an `else if`, the inner `if` needs no
braces around it. pawntidy adds the braces, except where they could change the program (see [When no
braces are added](pawntidy.md#when-no-braces-are-added)).

**Message:** ``the body of `<keyword>` is not in braces``, where `<keyword>` is `if`, `else`, `for`,
`while`, `do`, `case` or `default`. **Hint:** `` wrap it in `{ ... }` ``.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `check_switch_cases` | boolean | `true` | Also require braces around the bodies of `case` and `default`. |

### CodeLayout::RequireSameLineBraces

An opening brace at the end of its line keeps the header and its body together and saves a line per
block. It is the brace style of pawntidy.

Severity 3 (harsh). Themes: `core`, `layout`, `tidy`. No options.

**Reported:**

```sourcepawn
public void OnPluginStart()
{
	RegConsoleCmd("sm_hello", Command_Hello);
}
```

```
$ pawncritic --single-policy RequireSameLineBraces plugin.sp
plugin.sp:2:1: [3] the opening brace is not on the line of its statement  CodeLayout::RequireSameLineBraces
    2 | {
      | ^
      = hint: move `{` up to line 1

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	RegConsoleCmd("sm_hello", Command_Hello);
}
```

**Details:** reports an opening brace that is not on the line where the code before it ends. It
checks the braces of functions, conditions, loops, `switch`, enums, enum structs, methodmaps,
methodmap properties, structs, typesets and funcenums. A block that is a statement of its own, and a
block at the top level, are not checked.

**Message:** the summary. **Hint:** `` move `{` up to line <n> ``.

### CodeLayout::RequireTidyCode

Formatting by hand drifts, and a formatter keeps every file the same. This policy runs pawntidy on
the file and tells you whether it would change anything. It is the one-stop check for "is this file
formatted?".

Severity 1 (brutal). Themes: `core`, `layout`, `tidy`. A [native policy](#native-policies). No
options.

**Reported:** missing spaces around `=` and after `,`.

```sourcepawn
public void OnPluginStart() {
	int count=GetClientCount();
	PrintToServer("players: %d",count);
}
```

```
$ pawncritic --single-policy RequireTidyCode plugin.sp
plugin.sp:2:1: [1] pawntidy would change this file from line 2 on  CodeLayout::RequireTidyCode
    2 |     int count=GetClientCount();
      | ^
      = hint: run `pawntidy --write plugin.sp`

1 violation in 1 of 1 file (1 of severity 1).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	int count = GetClientCount();
	PrintToServer("players: %d", count);
}
```

**Details:**

- Formats the file with pawntidy and reports the first line pawntidy would change.
- The style comes from the nearest `.pawntidy.toml` or `pawntidy.toml` above the file, found as
  pawntidy finds it. pawncritic's `--config` and `--no-config` concern only `.pawncritic.toml` and
  do not change it.
- A file pawntidy cannot format, or an invalid pawntidy configuration file, makes the policy fail on
  that file, and pawncritic exits with status 2.

**Message:** `pawntidy would change this file from line <n> on`. **Hint:**
`` run `pawntidy --write <path>` ``.

## Complexity

Both policies count code lines: lines that hold a token other than a comment. Blank lines and lines
that hold only comments do not count.

### Complexity::ProhibitLongFiles

A long file mixes many concerns, and finding the part you are looking for takes searching instead of
reading. Split it into include files by topic, such as `scripting/include/<plugin>/menus.inc`, and
keep the plugin's `.sp` file for what ties them together.

Severity 3 (harsh). Themes: `complexity`, `core`, `philosophy`. Option: `max_lines`.

**Reported:** the default limit is 500 code lines. To keep the example short, it runs with this
`.pawncritic.toml` next to `plugin.sp`:

```toml
[policies."Complexity::ProhibitLongFiles"]
max_lines = 6
```

```sourcepawn
#include <sourcemod>

int g_iKills;
int g_iDeaths;

public void OnPluginStart() {
	HookEvent("player_death", Event_PlayerDeath);
}

public void Event_PlayerDeath(Event event, const char[] name, bool dontBroadcast) {
	g_iDeaths++;
}
```

```
$ pawncritic --single-policy ProhibitLongFiles plugin.sp
plugin.sp:1:1: [3] the file has 9 code lines, more than 6  Complexity::ProhibitLongFiles
    1 | #include <sourcemod>
      | ^
      = hint: split it into include files by topic

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:** the events live in `scripting/include/stats/events.inc`, and the plugin's file includes
its parts:

```sourcepawn
#include <sourcemod>
#include <stats/events>
```

**Details:** reports a file with more than `max_lines` code lines, at line 1.

**Message:** `the file has <n> code lines, more than <max>`. **Hint:**
`split it into include files by topic`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `max_lines` | integer | `500` | The most code lines a file may have. |

### Complexity::ProhibitLongFunctions

A long function does several things at once. Its reader has to keep all of them in mind, and a
change to one risks the others. Split it into smaller functions that each do one thing and are named
for it.

Severity 3 (harsh). Themes: `complexity`, `core`, `philosophy`. Option: `max_lines`.

**Reported:** the default limit is 50 code lines. To keep the example short, it runs with this
`.pawncritic.toml` next to `plugin.sp`:

```toml
[policies."Complexity::ProhibitLongFunctions"]
max_lines = 6
```

```sourcepawn
public void OnPluginStart() {
	RegConsoleCmd("sm_stats", Command_Stats);
	RegConsoleCmd("sm_top", Command_Top);
	RegAdminCmd("sm_resetstats", Command_Reset, ADMFLAG_ROOT);
	HookEvent("player_death", Event_PlayerDeath);
	HookEvent("round_end", Event_RoundEnd);
	LoadTranslations("stats.phrases");
}
```

```
$ pawncritic --single-policy ProhibitLongFunctions plugin.sp
plugin.sp:1:1: [3] `OnPluginStart` has 8 code lines, more than 6  Complexity::ProhibitLongFunctions
    1 | public void OnPluginStart() {
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: split it into smaller functions that each do one thing

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	RegisterCommands();
	HookEvents();
	LoadTranslations("stats.phrases");
}
```

**Details:**

- Checks functions, enum struct methods, methodmap methods, constructors, destructors, property
  accessors and operator overloads.
- The count runs from the first line of the function, its signature, to the line of its closing
  brace.
- A body that holds nothing but `MarkNativeAsOptional()` calls, such as the
  `__pl_<name>_SetNTVOptional` of a library's include file, is a list rather than logic and is not
  reported.

**Message:** ``` `<name>` has <n> code lines, more than <max> ```. A property accessor is named by
its property. **Hint:** `split it into smaller functions that each do one thing`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `max_lines` | integer | `50` | The most code lines a function may have. |

## Malicious

These policies report code that has no legitimate use in a plugin: backdoors, password theft,
attacks on the players' games and hidden text. They all have severity 5 and the themes `core`,
`malicious` and `security`. `pawncritic --theme malicious` runs exactly these six policies, which is
the quick way to vet a plugin from someone else (see [Vetting a third-party
plugin](vetting-plugins.md)).

### What the Malicious and Security policies can see

These policies read the code as written. They are heuristics, not a proof: a clean run does not make
a plugin safe. They see:

- **Literal text only.** That is a string literal, a `...` concatenation of string literals, or the
  template of a `Format()`, `FormatEx()`, `VFormat()`, `strcopy()`, `StrCat()` or `BuildPath()` call
  that fills the buffer passed, earlier in the same function. The entries below call this the
  **buffer template**. Text built any other way, read from a file or put together in another
  function is invisible to them.
- **Client input within one function only**, as described under
  [`Security::ProhibitCommandInjection`](#securityprohibitcommandinjection).
- **No included files and no macros.**

### Options that name functions

Many of these policies have options that list functions together with the positions of the arguments
they look at, counted from 1: `Name:N`, or `Name:N,M` for two positions. `"RenameFile:1,2"` means
both arguments of `RenameFile()` are paths.

- A method is named without its methodmap: `SetString:1` for `kv.SetString(...)`. Some options look
  at calls only and ignore methods of the same name: the `functions` of
  `Malicious::ProhibitClientSlowhack`, `Malicious::ProhibitDangerousDownloads`,
  `Security::ProhibitCommandInjection` and `Security::ProhibitServerControlCommands`, and the
  options whose entries below say "(not methods)".
- A constructor is named by its methodmap: `HTTPClient:1` for `new HTTPClient(...)`.
- A malformed entry makes the policy fail on every file with
  ``option `<option>`: `<entry>` is not of the form Name:N``.

To add a function, list the defaults and your entry. A list option replaces the default, it does not
add to it:

```toml
[policies."Security::ProhibitCommandInjection"]
functions = ["ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1", "ClientCommand:2",
             "FakeClientCommand:2", "FakeClientCommandEx:2", "MyLib_RunCommand:1"]
```

### Malicious::ProhibitClientSlowhack

`ClientCommand()` runs a command in the player's own game, not on the server. A plugin that binds
keys, writes or runs `.cfg` files, changes download settings or connects the player to another
server there harms the player beyond this server. This is known as "slowhacking".

Severity 5 (gentle). Themes: `core`, `malicious`, `security`. Options: `commands`, `functions`.

**Reported:**

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	ClientCommand(client, "bind mouse1 \"say I love this server\"");
}
```

```
$ pawncritic --single-policy ProhibitClientSlowhack plugin.sp
plugin.sp:2:24: [5] ClientCommand() runs `bind` on the player's game  Malicious::ProhibitClientSlowhack
    2 |     ClientCommand(client, "bind mouse1 \"say I love this server\"");
      |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** tell the player instead of changing their game:

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	PrintToChat(client, "Type !rules to read the server rules.");
}
```

**Details:** reports a literal command passed to a function of `functions` (directly or through its
buffer template) in which a command starts with a word in `commands`, ignoring case. Commands are
separated by `;` and line breaks. `FakeClientCommand()` runs on the server and is not checked.

**Message:** ``<function>() runs `<word>` on the player's game``. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `functions` | list | `["ClientCommand:2"]` | Functions that run a command on a client, with the command's position. |
| `commands` | list | `["bind", "unbind", "unbindall", "alias", "connect", "redirect", "writecfg", "exec", "cl_downloadfilter", "cl_allowupload", "cl_allowdownload", "motdfile", "motd_write", "con_logfile", "host_writeconfig", "quit", "exit"]` | Client commands a plugin must not run, in lower case. |

### Malicious::ProhibitDangerousDownloads

Every file in the downloads table lands in the player's game directory. A `.cfg` file there can
replace the player's own, such as `autoexec.cfg`, and run commands on their game. A program has no
place among game files at all.

Severity 5 (gentle). Themes: `core`, `malicious`, `security`. Options: `functions`, `extensions`,
`directories`.

**Reported:**

```sourcepawn
public void OnMapStart() {
	AddFileToDownloadsTable("sound/myserver/welcome.mp3");
	AddFileToDownloadsTable("cfg/autoexec.cfg");
}
```

```
$ pawncritic --single-policy ProhibitDangerousDownloads plugin.sp
plugin.sp:3:26: [5] players would download a `.cfg` file  Malicious::ProhibitDangerousDownloads
    3 |     AddFileToDownloadsTable("cfg/autoexec.cfg");
      |                             ^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** only the sound is downloaded:

```sourcepawn
public void OnMapStart() {
	AddFileToDownloadsTable("sound/myserver/welcome.mp3");
}
```

**Details:** reports a literal path passed to a function of `functions` (directly or through its
buffer template) that ends in one of `extensions` or starts with one of `directories`, ignoring
case. Before the comparison, `\` becomes `/` and a leading `./` is dropped.

**Message:** ``players would download a `<extension>` file`` or
``players would download a file in `<directory>` ``. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `functions` | list | `["AddFileToDownloadsTable:1"]` | Functions that add a download, with the path's position. |
| `extensions` | list | `[".cfg", ".exe", ".dll", ".so", ".dylib", ".bat", ".cmd", ".com", ".vbs", ".ps1", ".sh", ".scr", ".jar", ".lnk"]` | File name endings that are never downloaded. |
| `directories` | list | `["cfg/", "bin/"]` | Directories of the game that no download goes to. |

### Malicious::ProhibitObfuscatedStrings

Text written so that a reader cannot see it is how malicious plugins hide commands, ConVar names and
addresses from a review. Honest code has no reason to write `sv_cheats 1` as a list of numbers.

Severity 5 (gentle). Themes: `core`, `malicious`, `security`. Options: `min_length`, `min_escapes`,
`min_blob_length`.

**Reported:** the hint shows the hidden text.

```sourcepawn
public void OnPluginStart() {
	char command[] = { 115, 118, 95, 99, 104, 101, 97, 116, 115, 32, 49 };
	ServerCommand(command);
}
```

```
$ pawncritic --single-policy ProhibitObfuscatedStrings plugin.sp
plugin.sp:2:19: [5] the text is written as character codes  Malicious::ProhibitObfuscatedStrings
    2 |     char command[] = { 115, 118, 95, 99, 104, 101, 97, 116, 115, 32, 49 };
      |                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: write it as a string: "sv_cheats 1"

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	ServerCommand("sv_cheats 1");
}
```

**Details:** reports three ways of hiding text:

- **Character codes.** The initializer of a `char` (or old `String:`) array that holds at least
  `min_length` items, all integer literals of printable ASCII characters (32 to 126). Arrays of
  other types, such as lookup tables of `int`, are not checked. An array that ends in `0`, the
  terminator of a string, is not reported yet.
- **Escapes.** A string literal with at least `min_escapes` escapes (`\xNN`, `\NNN`) of printable
  ASCII characters. Escapes of control characters, such as the chat color codes `\x01` to `\x08`, do
  not count.
- **Blobs.** A string literal of at least `min_blob_length` characters that are all letters, digits,
  `+` and `/`, optionally followed by `=`: base64 or hex.

**Message:** `the text is written as character codes`, `printable characters are written as escapes`
or `a long base64 or hex string hides what it holds`. **Hint:** `write it as a string: "<text>"` or
`write them plainly: "<text>"` with the decoded text; none for a blob.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `min_length` | integer | `8` | How many printable integers make a char array text. |
| `min_escapes` | integer | `4` | How many escapes of printable characters make a string hidden. |
| `min_blob_length` | integer | `64` | The length from which a base64 or hex string counts as a blob. |

### Malicious::ProhibitPasswordConVars

Whoever knows the RCON password controls the server, and a plugin never needs it. Code that reads
it, sets it or names it in any other string is the classic way of stealing it or of locking the
owner out.

Severity 5 (gentle). Themes: `core`, `malicious`, `security`. Option: `convars`.

**Reported:** the plugin writes the password to a log file.

```sourcepawn
public void OnConfigsExecuted() {
	char password[64];
	FindConVar("rcon_password").GetString(password, sizeof(password));
	LogToFile("addons/sourcemod/logs/stats.log", "%s", password);
}
```

```
$ pawncritic --single-policy ProhibitPasswordConVars plugin.sp
plugin.sp:3:13: [5] the code names the ConVar `rcon_password`  Malicious::ProhibitPasswordConVars
    3 |     FindConVar("rcon_password").GetString(password, sizeof(password));
      |                ^^^^^^^^^^^^^^^
      = hint: a plugin never needs the RCON password; remove this

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** a plugin may read harmless ConVars such as `hostname`:

```sourcepawn
public void OnConfigsExecuted() {
	char hostname[64];
	FindConVar("hostname").GetString(hostname, sizeof(hostname));
	LogMessage("running on %s", hostname);
}
```

**Details:** reports a string literal, or a `...` concatenation of string literals, in which a word
(a run of letters, digits and `_`) is a ConVar of `convars`, ignoring case. `sv_password` is not in
the default list, because match plugins set it on purpose.

**Message:** ``the code names the ConVar `<name>` ``. **Hint:**
`a plugin never needs the RCON password; remove this`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `convars` | list | `["rcon_password", "sv_rcon_*"]` | ConVar names. A trailing `*` matches any rest. |

### Malicious::ProhibitSteamIdBackdoors

A function that compares a player with a SteamID written into the code and then grants admin rights
is a backdoor: the author keeps root access on every server that runs the plugin. Admins belong in
the server's admin configuration.

Severity 5 (gentle). Themes: `core`, `malicious`, `security`. Option: `grant_functions`.

**Reported:**

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	char auth[32];
	GetClientAuthId(client, AuthId_Steam2, auth, sizeof(auth));
	if (StrEqual(auth, "STEAM_0:1:12345678")) {
		SetUserFlagBits(client, ADMFLAG_ROOT);
	}
}
```

```
$ pawncritic --single-policy ProhibitSteamIdBackdoors plugin.sp
plugin.sp:4:21: [5] a hard-coded SteamID in a function that grants admin rights  Malicious::ProhibitSteamIdBackdoors
    4 |     if (StrEqual(auth, "STEAM_0:1:12345678")) {
      |                        ^^^^^^^^^^^^^^^^^^^^
      = hint: grant admin rights in the server's admin configuration

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** check the rights the server owner configured:

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	if (CheckCommandAccess(client, "sm_myplugin_admin", ADMFLAG_GENERIC)) {
		PrintToChat(client, "You can use the admin commands of this plugin.");
	}
}
```

**Details:** reports each string literal that is a SteamID (`STEAM_X:Y:Z`, `[U:1:N]` or a 64-bit
SteamID starting with `7656119`) in a function that also calls one of `grant_functions`. A SteamID
in a function that grants nothing is not reported.

**Message:** the summary. **Hint:** `grant admin rights in the server's admin configuration`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `grant_functions` | list | `["SetUserFlagBits", "AddUserFlags", "SetUserAdmin", "CreateAdmin", "SetAdminFlag", "SetFlag", "AdminInheritGroup", "InheritGroup", "BindAdminIdentity", "BindIdentity", "SetAdmGroupAddFlag", "SetAdmGroupImmunityLevel", "SetAdminImmunityLevel", "SetImmunityLevel"]` | Functions and methods that grant admin rights, by name. |

### Malicious::ProhibitVguiCommandBypass

The `cmd` key of a VGUI panel, such as the message of the day, is a command the player's game runs
when the panel closes. Plugins used it to run commands that `ClientCommand()` may not run, such as
binds or `connect`.

Severity 5 (gentle). Themes: `core`, `malicious`, `security`. Options: `panel_functions`,
`allowed_commands`.

**Reported:** closing the rules panel would connect the player to another server.

```sourcepawn
void ShowRules(int client) {
	KeyValues kv = new KeyValues("data");
	kv.SetString("title", "Rules");
	kv.SetString("type", "2");
	kv.SetString("msg", "https://example.com/rules");
	kv.SetString("cmd", "connect 203.0.113.7:27015");
	ShowVGUIPanel(client, "info", kv);
	delete kv;
}
```

```
$ pawncritic --single-policy ProhibitVguiCommandBypass plugin.sp
plugin.sp:6:15: [5] a VGUI panel with a `cmd` key runs a command on the player's game  Malicious::ProhibitVguiCommandBypass
    6 |     kv.SetString("cmd", "connect 203.0.113.7:27015");
      |                  ^^^^^
      = hint: leave the cmd key out or use closed_htmlpage

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** the panel has no `cmd` key:

```sourcepawn
void ShowRules(int client) {
	KeyValues kv = new KeyValues("data");
	kv.SetString("title", "Rules");
	kv.SetString("type", "2");
	kv.SetString("msg", "https://example.com/rules");
	ShowVGUIPanel(client, "info", kv);
	delete kv;
}
```

**Details:**

- Checks the functions that show a VGUI panel: those that call one of `panel_functions` or hold the
  string literal `VGUIMenu`, the name of the user message.
- In them, reports a string literal `cmd` that is an argument of a call, unless its value is an
  integer literal, a name of capital letters, digits and `_` (a macro or enum constant), a string of
  digits or one of `allowed_commands`. A value that is not a literal is reported. A `cmd` without a
  value (the last argument, with no call after it in the function) is not reported.
- The value is the next argument of the same call (`kv.SetString("cmd", <value>)`), or else the last
  argument of the next call in the function
  (`BfWriteString(bf, "cmd"); BfWriteString(bf, <value>);`).

**Message:** the summary. **Hint:** `leave the cmd key out or use closed_htmlpage`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `panel_functions` | list | `["ShowVGUIPanel"]` | Functions that show a VGUI panel. |
| `allowed_commands` | list | `["closed_htmlpage"]` | Harmless values of the `cmd` key. |

## Modern

These policies move a plugin to the syntax and API of SourcePawn 1.7 and later: typed declarations,
methodmaps and `delete`. Old code still compiles, but it hides types from the compiler, so mistakes
that modern code turns into compile errors only show at run time.

### Modern::ProhibitCloseHandle

`delete handle;` closes a handle and sets the variable to `null`, so the variable can never point to
a closed handle. `CloseHandle(handle)` leaves the stale value behind, and a second close or a later
use fails at run time.

Severity 4 (stern). Themes: `core`, `modern`. No options.

**Reported:** the hint is the replacement.

```sourcepawn
public void OnPluginEnd() {
	CloseHandle(g_hPlayers);
}
```

```
$ pawncritic --single-policy ProhibitCloseHandle plugin.sp
plugin.sp:2:2: [4] `CloseHandle()` is superseded by `delete`  Modern::ProhibitCloseHandle
    2 |     CloseHandle(g_hPlayers);
      |     ^^^^^^^^^^^^^^^^^^^^^^^
      = hint: delete g_hPlayers;

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
public void OnPluginEnd() {
	delete g_hPlayers;
}
```

**Details:** reports every call of `CloseHandle()`.

**Message:** the summary. **Hint:** `delete <argument>;`, with the argument of the call.

### Modern::ProhibitDeprecatedFunctions

SourceMod's include files mark natives and stocks that should no longer be used with
`#pragma deprecated`. Some of them no longer work, others have better replacements. This policy
knows them, so you get the advice without compiling.

Severity 4 (stern). Themes: `bugs`, `core`, `modern`. No options.

**Reported:** the message names the replacement.

```sourcepawn
public void OnClientAuthorized(int client, const char[] auth) {
	char steamId[32];
	GetClientAuthString(client, steamId, sizeof(steamId));
	LogMessage("%N has the Steam ID %s", client, steamId);
}
```

```
$ pawncritic --single-policy ProhibitDeprecatedFunctions plugin.sp
plugin.sp:3:2: [4] `GetClientAuthString()` is deprecated; use GetClientAuthId()  Modern::ProhibitDeprecatedFunctions
    3 |     GetClientAuthString(client, steamId, sizeof(steamId));
      |     ^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
public void OnClientAuthorized(int client, const char[] auth) {
	char steamId[32];
	GetClientAuthId(client, AuthId_Steam2, steamId, sizeof(steamId));
	LogMessage("%N has the Steam ID %s", client, steamId);
}
```

**Details:**

- Reports a call of `FindDataMapOffs`, `FindSendPropOffs`, `FloatAdd`, `FloatDiv`, `FloatMod`,
  `FloatMul`, `FloatSub`, `FormatUserLogText`, `GetAdmGroupImmunity`, `GetClientAuthString`,
  `GetClientListening`, `GetEntDataEnt`, `GetMaxClients`, `GetSoundDuration`, `GuessSDKVersion`,
  `IsPlayerInGame`, `IsSoundPrecached`, `IsValidHandle`, `LoadMaps`, `SQL_ConnectEx`,
  `SQL_QuoteString`, `SearchForClients`, `SetAdmGroupImmunity`, `SetClientListening`,
  `SetEntDataEnt`, `StrBreak`, `StrCompare`, `StrCopy`, `TF2_GetPlayerConditionFlags`,
  `TF2_GetPlayerResourceData`, `TF2_GetResourceEntity` and `TF2_SetPlayerResourceData`.
- Also reports the definition of a forward that SourceMod no longer calls or marks deprecated:
  `AskPluginLoad`, `OnLevelInit`, `OnServerCfg` and `TF2_OnGetHoliday`.
- The list is fixed in the policy, with advice in the policy's own words. The include files are not
  read.

**Message:** ``` `<name>()` is deprecated; <advice> ``` for a
call, ``` the forward `<name>` is deprecated; <advice> ``` for a forward. **Hint:** none.

### Modern::ProhibitFunctagAndFuncenum

`functag` and `funcenum` declare function types in old Pawn syntax. Since SourcePawn 1.7, `typedef`
declares one signature and `typeset` several.

Severity 4 (stern). Themes: `core`, `modern`. No options.

**Reported:**

```sourcepawn
functag public Action:RewardCallback(client, points);
```

```
$ pawncritic --single-policy ProhibitFunctagAndFuncenum plugin.sp
plugin.sp:1:1: [4] `functag` and `funcenum` are obsolete; use `typedef` and `typeset`  Modern::ProhibitFunctagAndFuncenum
    1 | functag public Action:RewardCallback(client, points);
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
typedef RewardCallback = function Action (int client, int points);
```

A `funcenum` with several signatures becomes a `typeset`:

```sourcepawn
typeset RewardCallback {
	function Action (int client, int points);
	function void (int client);
};
```

**Details:** reports every `functag` and `funcenum` declaration.

**Message:** the summary. **Hint:** none.

### Modern::ProhibitLegacyApiFunctions

Since SourceMod 1.7, handles have methodmaps: `ArrayList`, `StringMap`, `DataPack`, `ConVar`,
`Menu`, `KeyValues`, `File`, `Database`, `Event` and others. Their members say what they work on,
`list.Push(value)` instead of `PushArrayCell(list, value)`, and the compiler checks the handle's
type.

Severity 3 (harsh). Themes: `core`, `modern`. Options: `allow`, `extra`.

**Reported:** the message names the methodmap member.

```sourcepawn
ArrayList g_aNames;

void RememberName(const char[] name) {
	PushArrayString(g_aNames, name);
}
```

```
$ pawncritic --single-policy ProhibitLegacyApiFunctions plugin.sp
plugin.sp:4:2: [3] `PushArrayString()` is the old handle API; use `ArrayList.PushString()`  Modern::ProhibitLegacyApiFunctions
    4 |     PushArrayString(g_aNames, name);
      |     ^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
ArrayList g_aNames;

void RememberName(const char[] name) {
	g_aNames.PushString(name);
}
```

**Details:** reports a call of a function of the old handle API that has a methodmap member. The
policy knows the old functions of `ArrayList`, `ArrayStack`, `StringMap`, `DataPack`, `ConVar`,
`Menu`, `Panel`, `KeyValues`, `File`, `DirectoryListing`, `Database`, `DBResultSet`,
`GlobalForward`, `PrivateForward` and `Event`. The full list is in
`crates/pawncritic/policies/Modern/ProhibitLegacyApiFunctions.lua`.

**Message:** ``` `<name>()` is the old handle API; use `<replacement>` ```. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `allow` | list | `[]` | Functions not to report, with their exact case. |
| `extra` | list | `[]` | More functions to report, each as `"Name=Replacement"`, such as `"GetClientCookie=Cookie.Get()"`. Entries without `=` are ignored. |

For example, to keep `GetConVarInt()` for now and also report an old function of an extension:

```toml
[policies."Modern::ProhibitLegacyApiFunctions"]
allow = ["GetConVarInt"]
extra = ["GetClientCookie=Cookie.Get()"]
```

### Modern::ProhibitOldDeclarations

SourcePawn 1.7 introduced typed declarations: `int count = 0;` instead of `new count = 0;`,
`char name[64];` instead of `decl String:name[64];`, and functions with a return type. Old
declarations hide the type in a tag, and `decl` leaves the variable uninitialized, so it holds
whatever was in memory before.

Severity 4 (stern). Themes: `core`, `modern`. No options.

**Reported:** an old global, a function without a return type, and a `decl`.

```sourcepawn
new g_iCount = 0;

OnPluginStart() {
	decl String:name[64];
	GetClientName(1, name, sizeof(name));
}
```

```
$ pawncritic --single-policy ProhibitOldDeclarations plugin.sp
plugin.sp:1:1: [4] old-style declaration; declare the variable with its type, as in `int count = 0;`  Modern::ProhibitOldDeclarations
    1 | new g_iCount = 0;
      | ^^^^^^^^^^^^^^^^^

plugin.sp:3:1: [4] function `OnPluginStart` has no return type; declare it, as in `void OnPluginStart(...)`  Modern::ProhibitOldDeclarations
    3 | OnPluginStart() {
      | ^^^^^^^^^^^^^

plugin.sp:4:2: [4] `decl` declares a variable without initializing it; declare it with its type, as in `int count;`  Modern::ProhibitOldDeclarations
    4 |     decl String:name[64];
      |     ^^^^^^^^^^^^^^^^^^^^^

3 violations in 1 of 1 file (3 of severity 4).
```

**Fixed:**

```sourcepawn
int g_iCount = 0;

public void OnPluginStart() {
	char name[64];
	GetClientName(1, name, sizeof(name));
}
```

**Details:** reports old-style variable declarations (`new` and `decl`), and functions, forwards and
natives declared without a return type. With `#pragma newdecls required` the compiler rejects all of
these; see [`Modern::RequireNewDecls`](#modernrequirenewdecls).

**Message:** the summary for an old-style
declaration; ``` `decl` declares a variable without initializing it; declare it with its type, as in `int count;` ```
for a `decl`
declaration; ``` function `<name>` has no return type; declare it, as in `void <name>(...)` ``` for a function.
**Hint:** none.

### Modern::ProhibitOldTypeTags

Tags such as `Float:`, `String:` and `Handle:` are the type system of old Pawn. Since SourcePawn
1.7, a declaration names its type in front: `float speed`, `char name[64]`, `Handle timer`.

Severity 4 (stern). Themes: `core`, `modern`. No options.

**Reported:**

```sourcepawn
void SetSpeed(client, Float:speed) {
	SetEntPropFloat(client, Prop_Data, "m_flLaggedMovementValue", speed);
}
```

```
$ pawncritic --single-policy ProhibitOldTypeTags plugin.sp
plugin.sp:1:23: [4] old-style tag `Float:`; use the type `float`  Modern::ProhibitOldTypeTags
    1 | void SetSpeed(client, Float:speed) {
      |                       ^^^^^^

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
void SetSpeed(int client, float speed) {
	SetEntPropFloat(client, Prop_Data, "m_flLaggedMovementValue", speed);
}
```

**Details:** reports an old-style type tag in a declaration. Tag casts such as `Float:value` are
left to [`Modern::ProhibitTagCasts`](#modernprohibittagcasts).

**Message:** ``` old-style tag `<tag>:`; use the type `<type>` ```. `<type>` is `float` for `Float`,
`char` for `String`, `int` for `_`, and the tag's own name otherwise. **Hint:** none.

### Modern::ProhibitTagCasts

A tag cast like `_:value` or `Float:value` reinterprets a value without telling the reader what
happens. `view_as<int>(value)` does the same with the type spelled out.

Severity 4 (stern). Themes: `core`, `modern`. No options.

**Reported:**

```sourcepawn
void MoveToTeam(int client, TFTeam team) {
	ChangeClientTeam(client, _:team);
}
```

```
$ pawncritic --single-policy ProhibitTagCasts plugin.sp
plugin.sp:2:27: [4] old-style tag cast `_:`; use `view_as<int>(...)`  Modern::ProhibitTagCasts
    2 |     ChangeClientTeam(client, _:team);
      |                              ^^^^^^
      = hint: view_as<int>(team)

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
void MoveToTeam(int client, TFTeam team) {
	ChangeClientTeam(client, view_as<int>(team));
}
```

**Details:** reports every tag cast. The hint wraps the cast value as written, so a value that
already has parentheses gets a second pair; drop one when you apply it.

**Message:** ``` old-style tag cast `<tag>:`; use `view_as<<type>>(...)` ```. `<type>` is `float`
for `Float`, `int` for `_`, and the tag's own name otherwise. **Hint:** `view_as<<type>>(<value>)`,
with the cast value.

### Modern::RequireNewDecls

`#pragma newdecls required` makes the compiler reject old-style declarations. Once a plugin is
converted, the pragma keeps it in modern SourcePawn.

Severity 4 (stern). Themes: `core`, `modern`. Option: `check_includes`.

**Reported:** the violation is at line 1, because it concerns the whole file.

```sourcepawn
#pragma semicolon 1

#include <sourcemod>

public void OnPluginStart() {
	PrintToServer("loaded");
}
```

```
$ pawncritic --single-policy RequireNewDecls plugin.sp
plugin.sp:1:1: [4] the file lacks `#pragma newdecls required`  Modern::RequireNewDecls
    1 | #pragma semicolon 1
      | ^
      = hint: add `#pragma newdecls required` after the includes

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** the pragma comes after the includes, so that it does not apply to include files that
still use old declarations:

```sourcepawn
#pragma semicolon 1

#include <sourcemod>

#pragma newdecls required

public void OnPluginStart() {
	PrintToServer("loaded");
}
```

**Details:** reports a file without a `#pragma newdecls required` directive, at line 1. Include
files are checked only with `check_includes`, since a pragma in an include file also applies to the
files that include it.

**Message:** the summary. **Hint:** `` add `#pragma newdecls required` after the includes ``.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `check_includes` | boolean | `false` | Also require the pragma in include files. |

### Modern::RequireSemicolons

Without `#pragma semicolon 1`, the compiler accepts statements without a closing semicolon, and a
line break may end a statement where the author did not mean it to.

Severity 4 (stern). Themes: `core`, `modern`. Option: `check_includes`.

**Reported:**

```sourcepawn
#pragma newdecls required

#include <sourcemod>

public void OnPluginStart() {
	PrintToServer("loaded");
}
```

```
$ pawncritic --single-policy RequireSemicolons plugin.sp
plugin.sp:1:1: [4] the file lacks `#pragma semicolon 1`  Modern::RequireSemicolons
    1 | #pragma newdecls required
      | ^
      = hint: add `#pragma semicolon 1` at the top

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public void OnPluginStart() {
	PrintToServer("loaded");
}
```

**Details:** reports a file without a `#pragma semicolon 1` directive, at line 1. Include files are
checked only with `check_includes`.

**Message:** the summary. **Hint:** `` add `#pragma semicolon 1` at the top ``.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `check_includes` | boolean | `false` | Also require the pragma in include files. |

## NamingConventions

### NamingConventions::RequireGlobalPrefix

SourceMod plugins mark global variables with a prefix, `g_` by convention. A reader then sees at
once that a function touches state outside itself.

Severity 1 (brutal). Themes: `core`, `naming`. Option: `prefix`.

**Reported:** the constant is not reported.

```sourcepawn
ConVar enabled;
int roundCount;
const int MAX_ROUNDS = 30;
```

```
$ pawncritic --single-policy RequireGlobalPrefix plugin.sp
plugin.sp:1:8: [1] the global `enabled` lacks the prefix `g_`  NamingConventions::RequireGlobalPrefix
    1 | ConVar enabled;
      |        ^^^^^^^

plugin.sp:2:5: [1] the global `roundCount` lacks the prefix `g_`  NamingConventions::RequireGlobalPrefix
    2 | int roundCount;
      |     ^^^^^^^^^^

2 violations in 1 of 1 file (2 of severity 1).
```

**Fixed:**

```sourcepawn
ConVar g_cvEnabled;
int g_iRoundCount;
const int MAX_ROUNDS = 30;
```

**Details:** reports a global variable whose name does not start with `prefix`. Constants are not
reported: variables declared `const`, and names made only of upper case letters, digits and
underscores.

**Message:** ``` the global `<name>` lacks the prefix `<prefix>` ```. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `prefix` | string | `"g_"` | The prefix global variable names start with, with its exact case. |

## Philosophy

These policies encode a coding philosophy: leave early instead of nesting, give every value a
precise type, keep functions predictable, fail loudly, and choose names that carry meaning. Five of
the eight have severity 2 or 1, so a default run leaves them out.
`pawncritic --brutal --theme philosophy` runs all of them, together with the Complexity and
Variables policies, which share the theme.

### Philosophy::ProhibitBareHandle

A variable of type `Handle` can hold any handle, and the compiler cannot tell a menu from a timer.
Declared as `ArrayList`, `StringMap`, `ConVar`, `Menu` and so on, the variable only takes what it
should, and its methodmap members replace the old functions.

Severity 3 (harsh). Themes: `core`, `modern`, `philosophy`. Option: `extra`.

**Reported:** the violation is at the declaration, where the fix goes.

```sourcepawn
Handle g_hPlayers;

public void OnPluginStart() {
	g_hPlayers = CreateArray(ByteCountToCells(64));
}
```

```
$ pawncritic --single-policy ProhibitBareHandle plugin.sp
plugin.sp:1:8: [3] `g_hPlayers` stores the result of CreateArray(); declare it as `ArrayList g_hPlayers`  Philosophy::ProhibitBareHandle
    1 | Handle g_hPlayers;
      |        ^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
ArrayList g_hPlayers;

public void OnPluginStart() {
	g_hPlayers = new ArrayList(ByteCountToCells(64));
}
```

**Details:**

- Reports a variable declared as `Handle` that is given a handle with a methodmap.
- pawncritic looks at the value the variable gets: its initializer if that is a handle with a
  methodmap, otherwise the first such assignment (anywhere in the file for a global, in its function
  for a local). So `Handle g_hList = null;` followed by `g_hList = CreateArray();` is reported.
- A handle with a methodmap is a `new` of a methodmap, such as `new ArrayList()`, or the result of
  one of `CreateArray`, `CloneArray`, `ReadMapList`, `CreateStack`, `CreateTrie`,
  `CreateTrieSnapshot`, `CreateDataPack`, `CreateKeyValues`, `CreateMenu`, `CreatePanel`,
  `CreateConVar`, `FindConVar`, `OpenFile`, `OpenDirectory`, `SQL_Connect`, `SQL_DefConnect`,
  `SQL_ConnectCustom`, `SQL_Query`, `SQL_PrepareQuery`, `CreateGlobalForward`, `CreateForward`,
  `CreateEvent`, `CreateProfiler`, `CompileRegex`, `RegClientCookie`, `FindClientCookie`,
  `CreateTopMenu`, `GetAdminTopMenu` and `LoadGameConfigFile`.
- Handles without a methodmap, such as timers, HUD synchronizers and SDK calls, stay `Handle` and
  are not reported.

**Message:** ``` `<name>` stores the result of <source>; declare it as `<methodmap> <name>` ```,
where `<source>` is the call, such as `CreateArray()`, or `new <methodmap>()`. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `extra` | list | `[]` | More functions whose result has a methodmap, each as `"Function=Methodmap"`, for the natives of extensions. Entries of another shape are ignored. |

### Philosophy::ProhibitDeepNesting

Every level of nesting is one more condition the reader has to keep in mind, and deep nesting hides
bugs. Handle the edge cases at the top with guard clauses, skip loop passes with `continue`, and
give an inner part a function of its own.

Severity 3 (harsh). Themes: `core`, `philosophy`. Option: `max_depth`.

**Reported:** four levels inside the function, one more than allowed.

```sourcepawn
void HealTeam(int team) {
	for (int client = 1; client <= MaxClients; client++) {
		if (IsClientInGame(client)) {
			if (GetClientTeam(client) == team) {
				if (IsPlayerAlive(client)) {
					SetEntityHealth(client, 100);
				}
			}
		}
	}
}
```

```
$ pawncritic --single-policy ProhibitDeepNesting plugin.sp
plugin.sp:5:5: [3] nested 4 levels deep, more than 3  Philosophy::ProhibitDeepNesting
    5 |                 if (IsPlayerAlive(client)) {
      |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: return early, continue early or move the inner part into a function of its own

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:** the conditions become one guard clause:

```sourcepawn
void HealTeam(int team) {
	for (int client = 1; client <= MaxClients; client++) {
		if (!IsClientInGame(client) || GetClientTeam(client) != team || !IsPlayerAlive(client)) {
			continue;
		}
		SetEntityHealth(client, 100);
	}
}
```

**Details:** reports a condition, loop or `switch` nested more than `max_depth` levels deep inside
its function. An `else if` adds no level. Only the statement at the first level too deep is
reported, not the statements nested further inside it.

**Message:** `nested <n> levels deep, more than <max>`. **Hint:**
`return early, continue early or move the inner part into a function of its own`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `max_depth` | integer | `3` | The deepest nesting of conditions and loops allowed. |

### Philosophy::ProhibitElseAfterReturn

When the `if` branch ends in `return`, `break` or `continue`, the code after it only runs when the
condition is false anyway. An `else` around that code adds a level of indentation and nothing else.

Severity 2 (cruel). Themes: `core`, `philosophy`. No options.

**Reported:**

```sourcepawn
int PointsFor(int kills) {
	if (kills == 0) {
		return 0;
	} else {
		return kills * 10 + 5;
	}
}
```

```
$ pawncritic --single-policy ProhibitElseAfterReturn plugin.sp
plugin.sp:4:4: [2] `else` after `return`; drop it and unindent its body  Philosophy::ProhibitElseAfterReturn
    4 |     } else {
      |       ^^^^

1 violation in 1 of 1 file (1 of severity 2).
```

**Fixed:**

```sourcepawn
int PointsFor(int kills) {
	if (kills == 0) {
		return 0;
	}
	return kills * 10 + 5;
}
```

**Details:** reports the `else` of an `if` whose branch ends in `return`, `break` or `continue`. It
looks into nested blocks and past comments to find the last statement of the branch.

**Message:** ``` `else` after `<jump>`; drop it and unindent its body ```, where `<jump>` is
`return`, `break` or `continue`. **Hint:** none.

### Philosophy::ProhibitEmptyBlocks

A branch or loop that does nothing hides a case nobody decided about. Was something forgotten, or is
there really nothing to do? Remove the branch, handle the case, or say in a comment inside the
braces why nothing has to happen.

Severity 3 (harsh). Themes: `core`, `philosophy`. No options.

**Reported:**

```sourcepawn
public void OnClientDisconnect(int client) {
	if (IsFakeClient(client)) {
	}
	SaveStats(client);
}
```

```
$ pawncritic --single-policy ProhibitEmptyBlocks plugin.sp
plugin.sp:2:28: [3] the condition has an empty body  Philosophy::ProhibitEmptyBlocks
    2 |     if (IsFakeClient(client)) {
      |                               ^
      = hint: handle the case, remove it, or explain in a comment why nothing happens

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public void OnClientDisconnect(int client) {
	if (IsFakeClient(client)) {
		return;
	}
	SaveStats(client);
}
```

A comment inside the braces is fine too:

```sourcepawn
if (IsFakeClient(client)) {
	// bots have stats too; they are saved like everyone else's
}
```

**Details:** reports an empty pair of braces as the body of an `if`, `else`, `for`, `while`, `do`,
`case` or `default`. A body that holds a comment is not empty. Function bodies are not checked.

**Message:** `the <what> has an empty body`, where `<what>` is `condition` (for `if` and `else`),
`loop`, or `case` (for `case` and `default`). **Hint:**
`handle the case, remove it, or explain in a comment why nothing happens`.

### Philosophy::ProhibitHiddenGlobalMutation

A function should not surprise its caller. A function that returns nothing but writes to global
variables changes state the caller cannot see at the call. Return the new value and let the caller
store it, or pass what the function changes as a parameter.

Severity 2 (cruel). Themes: `core`, `philosophy`. No options.

**Reported:**

```sourcepawn
int g_iRound;

void NextRound() {
	g_iRound++;
}
```

```
$ pawncritic --single-policy ProhibitHiddenGlobalMutation plugin.sp
plugin.sp:3:6: [2] void function `NextRound` changes the global g_iRound  Philosophy::ProhibitHiddenGlobalMutation
    3 | void NextRound() {
      |      ^^^^^^^^^
      = hint: return the new value, or take what it changes as a parameter

1 violation in 1 of 1 file (1 of severity 2).
```

**Fixed:** the caller writes `g_iRound = NextRound(g_iRound);`, so the change is visible where it
happens:

```sourcepawn
int g_iRound;

int NextRound(int round) {
	return round + 1;
}
```

**Details:**

- Reports a function definition that is not `public`, returns `void` or has no return type, and
  assigns to, increments or decrements a global variable or an element of a global array.
- Writes to a local variable or parameter of the same name do not count.
- Writes to a field of a global (`g_data.field = 1;`) do not count, since an enum struct field
  cannot be told apart from a methodmap property without types.
- Public functions are callbacks and forwards, whose job is to change state, and are not reported.

**Message:** ``` void function `<function>` changes the global <name> ``` for one
global, ``` void function `<function>` changes the globals <name1>, <name2> ``` for several, in the order of
their first write. **Hint:** `return the new value, or take what it changes as a parameter`.

### Philosophy::ProhibitMeaninglessNames

Names carry the meaning, so that code reads like a sentence and needs no comment to explain it.
`tmp`, `foo`, `stuff` or `check()` make the reader work it out.

Severity 2 (cruel). Themes: `core`, `naming`, `philosophy`. Options: `variable_names`,
`function_names`, `short_names`.

**Reported:**

```sourcepawn
void handle(int client) {
	int tmp = GetClientHealth(client);
	PrintToChat(client, "health: %d", tmp);
}
```

```
$ pawncritic --single-policy ProhibitMeaninglessNames plugin.sp
plugin.sp:1:6: [2] `handle` says nothing about what the function does  Philosophy::ProhibitMeaninglessNames
    1 | void handle(int client) {
      |      ^^^^^^

plugin.sp:2:6: [2] `tmp` says nothing about what the variable holds  Philosophy::ProhibitMeaninglessNames
    2 |     int tmp = GetClientHealth(client);
      |         ^^^

2 violations in 1 of 1 file (2 of severity 2).
```

**Fixed:**

```sourcepawn
void ShowHealth(int client) {
	int health = GetClientHealth(client);
	PrintToChat(client, "health: %d", health);
}
```

**Details:** reports a variable, parameter or enum struct field whose name is in `variable_names`
(ignoring case), or is one letter that is not in `short_names` (exact case); and a function, enum
struct method or methodmap method whose name is in `function_names` (ignoring case).

**Message:** ``` `<name>` says nothing about what the variable holds ``` for a name in
`variable_names`; ``` the one-letter name `<name>` says nothing about what the variable holds ```
for a one-letter name; ``` `<name>` says nothing about what the function does ``` for a function.
**Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `variable_names` | list | `["tmp", "temp", "foo", "bar", "baz", "qux", "stuff", "thing", "things", "dummy", "blah", "asdf", "obj", "var", "val", "data2", "tmp2", "temp2"]` | Variable names to report. |
| `function_names` | list | `["check", "process", "handle", "doit", "do_it", "dostuff", "do_stuff", "func", "foo", "bar", "test", "temp", "helper"]` | Function names to report. |
| `short_names` | list | `["i", "j", "k", "x", "y", "z"]` | One-letter variable names that are fine. |

### Philosophy::RequireBooleanPredicateNames

A function that returns `bool` answers a question, and its name should ask it.
`IsValidClient(client)` says what `true` means; `ValidClient(client)` or `CheckClient(client)`
leaves the reader guessing.

Severity 1 (brutal). Themes: `core`, `naming`, `philosophy`. Option: `prefixes`.

**Reported:**

```sourcepawn
bool ValidClient(int client) {
	return client > 0 && client <= MaxClients && IsClientInGame(client);
}
```

```
$ pawncritic --single-policy RequireBooleanPredicateNames plugin.sp
plugin.sp:1:6: [1] `ValidClient` returns bool; name it as a question, like `IsValidClient`  Philosophy::RequireBooleanPredicateNames
    1 | bool ValidClient(int client) {
      |      ^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 1).
```

**Fixed:**

```sourcepawn
bool IsValidClient(int client) {
	return client > 0 && client <= MaxClients && IsClientInGame(client);
}
```

**Details:** reports a function definition that is not `public` and returns `bool` whose name,
without leading underscores, does not start with one of `prefixes` (ignoring case) followed by an
upper case letter, a digit, an underscore or the end of the name. `IsValid` and `has_item` pass,
`isvalid` and `Island` do not. Public functions are forwards and callbacks whose names the API
chooses.

**Message:** ``` `<name>` returns bool; name it as a question, like `Is<Name>` ```, with the first
letter of the name in upper case. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `prefixes` | list | `["is", "has", "have", "can", "could", "should", "was", "were", "will", "are", "does", "did", "needs", "contains", "exists", "allows", "supports", "matches", "equals", "try"]` | The words a predicate name may start with. |

### Philosophy::RequireEarlyExit

When the main work of a function sits inside an `if`, the reader has to find the end of the block to
learn what happens otherwise: nothing. Invert the condition, leave early, and do the work
unindented.

Severity 2 (cruel). Themes: `core`, `philosophy`. Option: `min_statements`.

**Reported:**

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	if (!IsFakeClient(client)) {
		LoadStats(client);
		ShowWelcome(client);
		CreateTimer(60.0, Timer_Advert, GetClientUserId(client));
	}
}
```

```
$ pawncritic --single-policy RequireEarlyExit plugin.sp
plugin.sp:2:2: [2] the rest of the body is wrapped in a condition; return or continue early instead  Philosophy::RequireEarlyExit
    2 |     if (!IsFakeClient(client)) {
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: invert the condition and `return` early, then do the work unindented

1 violation in 1 of 1 file (1 of severity 2).
```

**Fixed:**

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	if (IsFakeClient(client)) {
		return;
	}
	LoadStats(client);
	ShowWelcome(client);
	CreateTimer(60.0, Timer_Advert, GetClientUserId(client));
}
```

**Details:** reports the `if` that ends the body of a function or loop when it has no `else` and its
braced body holds at least `min_statements` statements. Comments do not count as statements.

**Message:** the summary. **Hint:**
``invert the condition and `return` early, then do the work unindented`` in a function, with
`` `continue` `` instead of `` `return` `` in a loop.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `min_statements` | integer | `3` | The smallest number of statements in the wrapping `if` that makes it a violation. |

## Security

These policies report code that puts the server, its files or its players at risk, by mistake or on
purpose. Some of it has legitimate uses, such as a plugin manager that loads plugins. Silence those
places with a [`no critic` annotation](pawncritic.md#annotations) and a reason.

What these policies can see, and how their options name functions, is described under [What the
Malicious and Security policies can see](#what-the-malicious-and-security-policies-can-see) and
[Options that name functions](#options-that-name-functions).

Several of them check paths. Paths start in the game directory, such as `tf/`; the paths of
`BuildPath(Path_SM, ...)` start in `addons/sourcemod/`.

### Security::ProhibitAbsolutePaths

File functions take paths relative to the game directory. An absolute path reaches anywhere on the
machine, outside the files the server owner gave the game.

Severity 4 (stern). Themes: `core`, `security`. Option: `file_functions`.

**Reported:**

```sourcepawn
public void OnPluginStart() {
	File log = OpenFile("/var/log/myplugin.log", "a");
	delete log;
}
```

```
$ pawncritic --single-policy ProhibitAbsolutePaths plugin.sp
plugin.sp:2:22: [4] the absolute path `/var/log/myplugin.log` reaches outside the game directory  Security::ProhibitAbsolutePaths
    2 |     File log = OpenFile("/var/log/myplugin.log", "a");
      |                         ^^^^^^^^^^^^^^^^^^^^^^^
      = hint: use a path relative to the game directory, or BuildPath(Path_SM, ...)

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	char path[PLATFORM_MAX_PATH];
	BuildPath(Path_SM, path, sizeof(path), "logs/myplugin.log");
	File log = OpenFile(path, "a");
	delete log;
}
```

**Details:** reports a literal path that starts with `/`, `\` or a drive letter (`C:`) and is passed
to a function of `file_functions`, directly or through its buffer template. `BuildPath()` itself is
not checked: its path always starts in the SourceMod directory.

**Message:** ``the absolute path `<path>` reaches outside the game directory``. **Hint:**
`use a path relative to the game directory, or BuildPath(Path_SM, ...)`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `file_functions` | list | `["OpenFile:1", "DeleteFile:1", "RenameFile:1,2", "FileExists:1", "DirExists:1", "CreateDirectory:1", "RemoveDir:1", "OpenDirectory:1", "FileSize:1", "GetFileTime:1", "SetFilePermissions:1", "FileToKeyValues:2", "KeyValuesToFile:2", "ImportFromFile:1", "ExportToFile:1", "LogToFile:1", "LogToFileEx:1", "SteamWorks_WriteHTTPResponseBodyToFile:2", "SetOutputFile:1"]` | Functions that take a path, with the positions of their paths. |

### Security::ProhibitCodeFileWrites

Plugins, extensions and Metamod:Source plugins are code the server runs. A plugin that writes,
deletes or renames them installs code nobody reviewed, removes other plugins such as an anti-cheat,
or updates itself behind the owner's back.

Severity 4 (stern). Themes: `core`, `security`. Options: `file_functions`, `write_functions`,
`code_extensions`, `code_directories`.

**Reported:**

```sourcepawn
public void OnPluginStart() {
	DeleteFile("addons/sourcemod/plugins/anticheat.smx");
}
```

```
$ pawncritic --single-policy ProhibitCodeFileWrites plugin.sp
plugin.sp:2:13: [4] DeleteFile() changes the code file `addons/sourcemod/plugins/anticheat.smx`  Security::ProhibitCodeFileWrites
    2 |     DeleteFile("addons/sourcemod/plugins/anticheat.smx");
      |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** a plugin may manage its own data files:

```sourcepawn
public void OnPluginStart() {
	DeleteFile("addons/sourcemod/data/myplugin/cache.txt");
}
```

**Details:**

- Reports a literal path that reaches a function of `write_functions` (directly or as the template
  of the buffer passed) and is a file of code.
- A file of code ends in one of `code_extensions` or is in one of `code_directories`, ignoring case,
  after `..` and `.` parts are resolved and `\` became `/`.
- `OpenFile()` counts only when its mode is a literal with `w`, `a` or `+`, or is not a literal.

**Message:** ``<function>() changes the code file `<path>` ``. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `file_functions` | list | as for [`Security::ProhibitAbsolutePaths`](#securityprohibitabsolutepaths) | Functions that take a path. |
| `write_functions` | list | `["OpenFile:1", "DeleteFile:1", "RenameFile:1,2", "CreateDirectory:1", "RemoveDir:1", "KeyValuesToFile:2", "ExportToFile:1", "LogToFile:1", "LogToFileEx:1", "SteamWorks_WriteHTTPResponseBodyToFile:2", "SetOutputFile:1"]` | Those of them that write, delete or rename. A function must also be listed in `file_functions`, or it is never seen. |
| `code_extensions` | list | `[".smx", ".so", ".dll", ".dylib", ".vdf"]` | File name endings of code. |
| `code_directories` | list | `["addons/sourcemod/plugins/", "addons/sourcemod/extensions/", "addons/sourcemod/bin/", "addons/metamod/", "bin/"]` | Directories of code below the game directory. |

### Security::ProhibitCommandInjection

A player's name, chat message or command argument may hold `;` followed by further commands. Put
into a server or client command, it runs whatever the player wrote, with the rights of the server. A
map argument of `cp_badlands; rcon_password hacked` changes more than the map.

Severity 5 (gentle). Themes: `core`, `security`. Options: `functions`, `sources`, `forwards`,
`propagators`.

**Reported:**

```sourcepawn
public Action Command_Map(int client, int args) {
	char map[64];
	GetCmdArg(1, map, sizeof(map));
	ServerCommand("changelevel %s", map);
	return Plugin_Handled;
}
```

```
$ pawncritic --single-policy ProhibitCommandInjection plugin.sp
plugin.sp:4:34: [5] `map` holds client input and goes into ServerCommand()  Security::ProhibitCommandInjection
    4 |     ServerCommand("changelevel %s", map);
      |                                     ^^^
      = hint: refer to players by `#userid`; do not pass free text

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** check the argument, then use a function instead of a command line. Refer to players by
user id (`#%d` with `GetClientUserId()`), never by name.

```sourcepawn
public Action Command_Map(int client, int args) {
	char map[64];
	GetCmdArg(1, map, sizeof(map));
	if (!IsMapValid(map)) {
		ReplyToCommand(client, "Unknown map.");
		return Plugin_Handled;
	}
	ForceChangeLevel(map, "sm_map");
	return Plugin_Handled;
}
```

The fixed code is no longer reported because the map name no longer reaches a command function.
Testing the argument with `IsMapValid()` alone would not satisfy the policy: no function removes
client input from a variable.

**Details:**

- Reports a variable holding client input that is an argument of a function of `functions`, at the
  command's position or after it.
- **How client input is tracked.** Within one function, in the order of the code (loops are not
  followed back), a plain variable holds client input:
  - once a call of `sources` filled it (its buffer argument);
  - when it is a parameter of a function named in `forwards`, at the given position;
  - once a call of `propagators` wrote into it while one of its other arguments held client input.
- Array elements, fields and return values are not tracked, and no function removes client input
  from a variable.

**Message:** ``` `<name>` holds client input and goes into <function>() ```. **Hint:**
``refer to players by `#userid`; do not pass free text``.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `functions` | list | `["ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1", "ClientCommand:2", "FakeClientCommand:2", "FakeClientCommandEx:2"]` | Functions that run a command, with the command's position. |
| `sources` | list | `["GetCmdArg:2", "GetCmdArgString:1", "GetClientName:2", "GetClientInfo:3", "GetEventString:3", "GetClientCookie:3"]` | Calls (not methods) that fill a buffer with client input, with the buffer's position. |
| `forwards` | list | `["OnClientSayCommand:3", "OnClientSayCommand_Post:3"]` | Functions whose parameters hold client input, with their positions. |
| `propagators` | list | `["Format:1", "FormatEx:1", "VFormat:1", "strcopy:1", "StrCat:1", "BuildPath:2"]` | Calls (not methods) that write their other arguments into a buffer, with its position. |

### Security::ProhibitHardcodedEndpoints

A plugin that sends data to an address written into its code reports to its author. The server owner
cannot see or change where the data goes, and it is how stolen passwords and player data leave a
server. Read addresses from a ConVar or a configuration file the owner controls.

Severity 4 (stern). Themes: `core`, `security`. Option: `natives`.

**Reported:**

```sourcepawn
void SendStats(const char[] body) {
	Handle request = SteamWorks_CreateHTTPRequest(k_EHTTPMethodPOST, "http://stats.example.com/submit");
	SteamWorks_SetHTTPRequestRawPostBody(request, "application/json", body, strlen(body));
	SteamWorks_SendHTTPRequest(request);
}
```

```
$ pawncritic --single-policy ProhibitHardcodedEndpoints plugin.sp
plugin.sp:2:67: [4] SteamWorks_CreateHTTPRequest() connects to `http://stats.example.com/submit`, written into the code  Security::ProhibitHardcodedEndpoints
    2 |     Handle request = SteamWorks_CreateHTTPRequest(k_EHTTPMethodPOST, "http://stats.example.com/submit");
      |                                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: read the address from a ConVar or a config file

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** the address comes from a ConVar the owner sets:

```sourcepawn
ConVar g_cvStatsUrl;

void SendStats(const char[] body) {
	char url[256];
	g_cvStatsUrl.GetString(url, sizeof(url));
	Handle request = SteamWorks_CreateHTTPRequest(k_EHTTPMethodPOST, url);
	SteamWorks_SetHTTPRequestRawPostBody(request, "application/json", body, strlen(body));
	SteamWorks_SendHTTPRequest(request);
}
```

**Details:** reports a literal address passed to a function or constructor of `natives`, directly or
through its buffer template, unless it is empty or starts with `%` (a placeholder, such as a host
read from a ConVar). Host names, IP addresses and URLs are all reported. Auto-updaters
(`Updater_AddPlugin()`) are not in the default list.

**Message:** ``<function>() connects to `<address>`, written into the code``. **Hint:**
`read the address from a ConVar or a config file`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `natives` | list | `["SocketConnect:5", "SocketSendTo:4", "SteamWorks_CreateHTTPRequest:2", "System2HTTPRequest:2", "System2FTPRequest:2", "System2_GetPage:2", "System2_DownloadFile:2", "HTTPClient:1", "HTTPRequest:1"]` | Functions and constructors that take an address, with its position. |

### Security::ProhibitNonLiteralFormat

A format function reads every `%` of its format as a specifier. When the format is a variable, such
as a player's chat message, a player who writes `%s` makes the call read arguments that are not
there. The call then fails or prints what it should not.

Severity 4 (stern). Themes: `bugs`, `core`, `security`. Option: `functions`.

**Reported:** the player's message is used as the format.

```sourcepawn
public void OnClientSayCommand_Post(int client, const char[] command, const char[] message) {
	PrintToChatAll(message);
}
```

```
$ pawncritic --single-policy ProhibitNonLiteralFormat plugin.sp
plugin.sp:2:17: [4] the format of PrintToChatAll() is `message`, not fixed text  Security::ProhibitNonLiteralFormat
    2 |     PrintToChatAll(message);
      |                    ^^^^^^^
      = hint: pass it as an argument: "%s", message

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** the message is an argument:

```sourcepawn
public void OnClientSayCommand_Post(int client, const char[] command, const char[] message) {
	PrintToChatAll("%s", message);
}
```

**Details:** reports the format argument of a call of `functions` when no argument follows it and it
is not fixed text. Fixed text is a string literal, a name of capital letters, digits and `_` (most
likely a macro of a string), or a `...` concatenation of those. A format followed by arguments is
not reported.

**Message:** ``the format of <function>() is `<text>`, not fixed text``. **Hint:**
`pass it as an argument: "%s", <text>`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `functions` | list | see below | Format functions, with the format's position. |

The default of `functions` holds the format functions of SourceMod (`PrintToChat:2`,
`PrintToChatAll:1`, `PrintCenterText:2`, `PrintCenterTextAll:1`, `PrintHintText:2`,
`PrintHintTextToAll:1`, `PrintToConsole:2`, `PrintToConsoleAll:1`, `PrintToServer:1`,
`ReplyToCommand:2`, `ShowActivity:2`, `ShowActivity2:3`, `ShowActivityEx:3`, `LogMessage:1`,
`LogError:1`, `LogAction:3`, `LogToFile:2`, `LogToFileEx:2`, `LogToGame:1`, `Format:3`,
`FormatEx:3`, `ThrowError:1`, `ThrowNativeError:2`, `SetFailState:1`, `ServerCommand:1`,
`ServerCommandEx:3`, `InsertServerCommand:1`, `ClientCommand:2`, `FakeClientCommand:2`,
`FakeClientCommandEx:2`, `KickClient:2`, `KickClientEx:2`, `ShowHudText:3`, `ShowSyncHudText:3`), of
the color libraries morecolors, colors and multicolors (`CPrintToChat:2`, `CPrintToChatAll:1`,
`CPrintToChatEx:3`, `CPrintToChatAllEx:2`, `CReplyToCommand:2`, `CShowActivity:2`,
`CShowActivity2:3`, `CShowActivityEx:3`) and of `etf2l.inc` (`ETF2L_PrintToChat:2`,
`ETF2L_PrintToChatEx:3`, `ETF2L_PrintToChatAll:1`, `ETF2L_PrintToChatAllEx:2`,
`ETF2L_PrintToTeam:2`, `ETF2L_PrintToTeamEx:3`, `ETF2L_PrintToAdmins:2`, `ETF2L_PrintToAdminsEx:3`,
`ETF2L_ShowActivity:2`, `ETF2L_PrintToServer:1`, `ETF2L_PrintToConsole:2`, `ETF2L_Reply:2`,
`ETF2L_ReplyEx:3`, `ETF2L_LogDebug:1`, `ETF2L_LogInfo:1`, `ETF2L_LogWarning:1`, `ETF2L_LogError:1`).

### Security::ProhibitPathTraversal

A path with enough `..` parts climbs out of the game directory and reaches any file the server's
user may read or write, such as SSH keys or other servers' configs.

Severity 5 (gentle). Themes: `core`, `security`. Option: `file_functions`.

**Reported:** `BuildPath(Path_SM, ...)` starts in `addons/sourcemod/`, and four `..` climb above the
game directory.

```sourcepawn
public void OnPluginStart() {
	char path[PLATFORM_MAX_PATH];
	BuildPath(Path_SM, path, sizeof(path), "../../../../.ssh/authorized_keys");
	File file = OpenFile(path, "r");
	delete file;
}
```

```
$ pawncritic --single-policy ProhibitPathTraversal plugin.sp
plugin.sp:3:41: [5] the path `../../../../.ssh/authorized_keys` climbs out of the game directory  Security::ProhibitPathTraversal
    3 |     BuildPath(Path_SM, path, sizeof(path), "../../../../.ssh/authorized_keys");
      |                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:**

```sourcepawn
public void OnPluginStart() {
	char path[PLATFORM_MAX_PATH];
	BuildPath(Path_SM, path, sizeof(path), "configs/myplugin.cfg");
	File file = OpenFile(path, "r");
	delete file;
}
```

**Details:**

- Reports a literal path whose `..` parts climb above the game directory: one passed to a function
  of `file_functions` (directly or through its buffer template), and the path of every
  `BuildPath()`.
- `BuildPath(Path_SM, path, sizeof(path), "../../cfg/x.cfg")` stays inside the game directory and is
  not reported; `"../../../x"` climbs out.
- `/` and `\` both separate parts.
- A template that starts with `%`, and `BuildPath()` with a path type other than `Path_SM`, start in
  an unknown place and are not checked. Absolute paths are left to
  [`Security::ProhibitAbsolutePaths`](#securityprohibitabsolutepaths).

**Message:** ``the path `<path>` climbs out of the game directory``. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `file_functions` | list | as for [`Security::ProhibitAbsolutePaths`](#securityprohibitabsolutepaths) | Functions that take a path. |

### Security::ProhibitSecretFileAccess

Some files of a server hold database passwords, admin rights and usually the RCON password.
SourceMod's API gives a plugin what it needs from them, such as a database connection, without the
plugin reading the file.

Severity 4 (stern). Themes: `core`, `security`. Options: `file_functions`, `secret_files`.

**Reported:**

```sourcepawn
public void OnPluginStart() {
	char path[PLATFORM_MAX_PATH];
	BuildPath(Path_SM, path, sizeof(path), "configs/databases.cfg");
	KeyValues kv = new KeyValues("Databases");
	kv.ImportFromFile(path);
	delete kv;
}
```

```
$ pawncritic --single-policy ProhibitSecretFileAccess plugin.sp
plugin.sp:3:41: [4] the plugin opens `databases.cfg`  Security::ProhibitSecretFileAccess
    3 |     BuildPath(Path_SM, path, sizeof(path), "configs/databases.cfg");
      |                                            ^^^^^^^^^^^^^^^^^^^^^^^
      = hint: use SourceMod's API (SQL_Connect, the admin functions) instead of the file

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** connect through the database configuration by name:

```sourcepawn
public void OnPluginStart() {
	Database.Connect(OnDatabaseConnected, "myplugin");
}
```

**Details:** reports a literal path passed to a function of `file_functions` (directly or through
its buffer template), or the path of a `BuildPath()`, whose last part is one of `secret_files`,
ignoring case.

**Message:** ``the plugin opens `<name>` ``. **Hint:**
`use SourceMod's API (SQL_Connect, the admin functions) instead of the file`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `file_functions` | list | as for [`Security::ProhibitAbsolutePaths`](#securityprohibitabsolutepaths) | Functions that take a path. |
| `secret_files` | list | `["databases.cfg", "admins.cfg", "admins_simple.ini", "admin_groups.cfg", "admin_overrides.cfg", "core.cfg", "server.cfg"]` | File names that hold credentials or admin rights. |

### Security::ProhibitServerControlCommands

Loading and unloading plugins and extensions, RCON and shutting the server down are the owner's
decisions. A plugin that runs these commands can remove an anti-cheat, load code nobody reviewed, or
take the server down.

Severity 4 (stern). Themes: `core`, `security`. Options: `functions`, `commands`.

**Reported:**

```sourcepawn
public Action Command_Update(int client, int args) {
	ServerCommand("sm plugins unload anticheat");
	return Plugin_Handled;
}
```

```
$ pawncritic --single-policy ProhibitServerControlCommands plugin.sp
plugin.sp:2:16: [4] ServerCommand() runs `sm plugins unload`  Security::ProhibitServerControlCommands
    2 |     ServerCommand("sm plugins unload anticheat");
      |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:**

```sourcepawn
public Action Command_Update(int client, int args) {
	ReplyToCommand(client, "Ask the server owner to update the plugins.");
	return Plugin_Handled;
}
```

**Details:** reports a literal command passed to a function of `functions` (directly or through its
buffer template) in which a command is one of `commands`, or starts with one of them followed by a
space. Commands are separated by `;` or a line break. The comparison ignores case and counts any run
of whitespace as one space. In a plugin manager that loads plugins on purpose, silence the line with
an annotation.

**Message:** ``<function>() runs `<command>` ``. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `functions` | list | `["ServerCommand:1", "ServerCommandEx:3", "InsertServerCommand:1"]` | Functions that run a server command, with the command's position. |
| `commands` | list | `["sm plugins load", "sm plugins unload", "sm plugins unload_all", "sm plugins reload", "sm plugins refresh", "sm exts load", "sm exts unload", "sm exts reload", "meta load", "meta unload", "plugin_load", "plugin_unload", "rcon", "sm_rcon", "quit", "exit", "_restart", "killserver"]` | Commands, with their first words. |

### Security::ProhibitShellExecution

A function that runs a program on the server's machine runs it with the rights of the game server.
That is everything the server's user may do, far beyond the game.

Severity 4 (stern). Themes: `core`, `security`. Option: `functions`.

**Reported:**

```sourcepawn
public void OnMapEnd() {
	System2_Execute(g_sOutput, sizeof(g_sOutput), "tar -czf /tmp/demos.tgz tf/demos");
}
```

```
$ pawncritic --single-policy ProhibitShellExecution plugin.sp
plugin.sp:2:2: [4] System2_Execute() runs a program on the server's machine  Security::ProhibitShellExecution
    2 |     System2_Execute(g_sOutput, sizeof(g_sOutput), "tar -czf /tmp/demos.tgz tf/demos");
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 violation in 1 of 1 file (1 of severity 4).
```

**Fixed:** leave such jobs to the machine's own tools:

```sourcepawn
public void OnMapEnd() {
	LogMessage("the map ended; the demos are archived by the server's own cron job");
}
```

**Details:** reports every call of a function of `functions`. A method of the same name is not
reported.

**Message:** `<function>() runs a program on the server's machine`. **Hint:** none.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `functions` | list | `["System2_Execute", "System2_ExecuteThreaded", "System2_ExecuteFormatted", "System2_ExecuteFormattedThreaded"]` | Functions that run a program, by name. |

### Security::ProhibitSqlInjection

A player's name or command argument put into a query unescaped can end the string and add SQL of its
own. A name of `x' OR '1'='1` turns a lookup into a dump of the table, and worse queries delete it.

Severity 5 (gentle). Themes: `bugs`, `core`, `security`. Options: `format_functions`, `sources`,
`forwards`, `propagators`, `sanitizers`.

**Reported:**

```sourcepawn
public Action Command_Search(int client, int args) {
	char name[64];
	GetCmdArg(1, name, sizeof(name));
	char query[256];
	Format(query, sizeof(query), "SELECT points FROM players WHERE name = '%s'", name);
	g_db.Query(OnSearchDone, query, GetClientUserId(client));
	return Plugin_Handled;
}
```

```
$ pawncritic --single-policy ProhibitSqlInjection plugin.sp
plugin.sp:5:79: [5] `name` holds client input and goes into a query unescaped  Security::ProhibitSqlInjection
    5 |     Format(query, sizeof(query), "SELECT points FROM players WHERE name = '%s'", name);
      |                                                                                  ^^^^
      = hint: format the query with Database.Format()

1 violation in 1 of 1 file (1 of severity 5).
```

**Fixed:** `Database.Format()` escapes the `%s` arguments:

```sourcepawn
public Action Command_Search(int client, int args) {
	char name[64];
	GetCmdArg(1, name, sizeof(name));
	char query[256];
	g_db.Format(query, sizeof(query), "SELECT points FROM players WHERE name = '%s'", name);
	g_db.Query(OnSearchDone, query, GetClientUserId(client));
	return Plugin_Handled;
}
```

**Details:**

- Reports a variable holding client input that a call (not a method) of `format_functions` formats
  with `%s` into a literal template with one of the words `SELECT`, `INSERT`, `UPDATE`, `DELETE` or
  `REPLACE`, ignoring case.
- Arguments are paired with the template's conversions in order; `%T` takes two arguments, `%%`
  none.
- Client input is tracked as for
  [`Security::ProhibitCommandInjection`](#securityprohibitcommandinjection), except that a buffer
  written by one of `sanitizers` no longer holds client input.
- `Database.Format()` and `SQL_FormatQuery()`, which escape their `%s` arguments, are not format
  functions here.

**Message:** ``` `<name>` holds client input and goes into a query unescaped ```. **Hint:**
`format the query with Database.Format()`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `format_functions` | list | `["Format:3", "FormatEx:3"]` | Functions that format text, with the template's position. |
| `sources`, `forwards`, `propagators` | list | as for [`Security::ProhibitCommandInjection`](#securityprohibitcommandinjection) | Where client input comes from and how it spreads. |
| `sanitizers` | list | `["SQL_EscapeString:3", "Escape:2"]` | Calls and methods that write escaped text into a buffer, with its position. |

## Variables

A variable declared at the top of a function, far from where it is used, makes the reader keep it in
mind all the way down. Declared where it is first needed, it is visible exactly where it matters.

Both policies look only at declarations they can move without changing what the code does:

- local variables declared in a declaration statement directly inside braces, not in a `for` header;
- each variable of a declaration such as `int a, b;` on its own;
- without an initializer, or with a constant one: a literal, a unary operator (`-`, `!`, `~`)
  applied to a constant, `null`, or an array literal of constants such as `{0, 0, 0}`.

A variable is skipped when it is never used (the compiler warns about that), when its name is
declared again in the same block or in a block nested in it, or when the block holds code the parser
cannot read.

### Variables::RequireLateDeclarations

A variable declared at the top of a function, and first used after an early return, makes the reader
keep it in mind through code that never needs it. Declared just before its first use, it also cannot
be read by mistake before it holds its real value.

Severity 3 (harsh). Themes: `core`, `philosophy`, `variables`. Option: `min_statements`.

**Reported:** the `if` comes between the declaration and the first use.

```sourcepawn
public Action Command_Points(int client, int args) {
	int points = 0;
	if (client == 0) {
		return Plugin_Handled;
	}
	points = GetPoints(client);
	PrintToChat(client, "You have %d points.", points);
	return Plugin_Handled;
}
```

```
$ pawncritic --single-policy RequireLateDeclarations plugin.sp
plugin.sp:2:6: [3] `points` is declared 1 statement before its first use  Variables::RequireLateDeclarations
    2 |     int points = 0;
      |         ^^^^^^^^^^
      = hint: declare it just before line 6, where it is first used

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public Action Command_Points(int client, int args) {
	if (client == 0) {
		return Plugin_Handled;
	}
	int points = GetPoints(client);
	PrintToChat(client, "You have %d points.", points);
	return Plugin_Handled;
}
```

**Details:** reports a variable when at least `min_statements` statements come between its
declaration and the statement of its first use in the same block. Declarations of the same kind
(without an initializer or with a constant one), comments and directives in between do not count. A
variable that [`Variables::RequireNarrowDeclarations`](#variablesrequirenarrowdeclarations) would
report is not reported here, whether that policy runs or not.

**Message:** ``` `<name>` is declared <n> statements before its first use ``` (`1 statement` for
one). **Hint:** `declare it just before line <line>, where it is first used`.

**Options:**

| Option | Type | Default | Meaning |
|---|---|---|---|
| `min_statements` | integer | `1` | The smallest number of statements between declaration and first use that makes it a violation. |

### Variables::RequireNarrowDeclarations

A variable used only inside an `if` or a loop belongs there. Declared in that block, it shows the
reader that nothing outside depends on it, and it cannot leak a stale value into the code after the
block.

Severity 3 (harsh). Themes: `core`, `philosophy`, `variables`. No options.

**Reported:** `name` is used only inside the `if`.

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	char name[MAX_NAME_LENGTH];
	if (IsFakeClient(client)) {
		GetClientName(client, name, sizeof(name));
		LogMessage("bot %s joined", name);
	}
}
```

```
$ pawncritic --single-policy RequireNarrowDeclarations plugin.sp
plugin.sp:2:7: [3] `name` is used only inside the block at line 3  Variables::RequireNarrowDeclarations
    2 |     char name[MAX_NAME_LENGTH];
      |          ^^^^^^^^^^^^^^^^^^^^^
      = hint: declare it inside that block, just before line 4

1 violation in 1 of 1 file (1 of severity 3).
```

**Fixed:**

```sourcepawn
public void OnClientPostAdminCheck(int client) {
	if (IsFakeClient(client)) {
		char name[MAX_NAME_LENGTH];
		GetClientName(client, name, sizeof(name));
		LogMessage("bot %s joined", name);
	}
}
```

**Details:**

- Reports a variable whose every use is inside one braced body of an `if`, `else` or loop, and names
  the innermost such body.
- A loop body is named only when no value has to survive from one pass to the next. That is the case
  when the first use in the body, in a statement that runs on every pass, is either:
  - a plain assignment `x = ...` that does not read `x`, or
  - the array passed to a call as the buffer it fills, followed by its `sizeof()` and used nowhere
    else among the arguments.
- An array declared in a loop body is filled with zeros on every pass. In a hot loop, keep it
  outside and silence the policy there with an [annotation](pawncritic.md#annotations).

**Message:** ``` `<name>` is used only inside the block at line <n> ```. **Hint:**
`declare it inside that block, just before line <line>`.

## Native policies

Two built-in policies are written in Rust instead of Lua, because they need what the Lua API does
not offer: [`Core::RequireParsableCode`](#corerequireparsablecode) reads the parser's errors, and
[`CodeLayout::RequireTidyCode`](#codelayoutrequiretidycode) runs pawntidy. They have no Lua time or
memory [limits](pawncritic.md#the-limits-table), but the watchdog still abandons one that runs on
one file for longer than twice `timeout_ms` plus 2 seconds (see
[Limits](writing-policies.md#limits)). `--explain` shows `Source: built-in native policy` for them.
