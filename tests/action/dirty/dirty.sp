#pragma semicolon 1
#pragma newdecls required

#include <sourcemod>

Handle g_hTimer;

public void OnPluginEnd() {
    if (g_hTimer != null) CloseHandle(g_hTimer);
	g_hTimer = null;
}
