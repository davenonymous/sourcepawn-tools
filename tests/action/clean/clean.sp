#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

public Plugin myinfo = {
	name = "clean",
	author = "davenonymous",
	description = "A plugin the action must find nothing in",
	version = "1.0.0",
	url = ""
};

public void OnPluginStart() {
	PrintToServer("clean");
}
