// policy: Security::ProhibitSecretFileAccess
// expect: 5 6 7 8
void f() {
	char path[PLATFORM_MAX_PATH];
	BuildPath(Path_SM, path, sizeof(path), "configs/databases.cfg");
	OpenFile("cfg/Server.cfg", "r");
	FileToKeyValues(kv, "addons/sourcemod/configs/admins.cfg");
	Format(path, sizeof(path), "%s/admins_simple.ini", "addons/sourcemod/configs");
	OpenFile(path, "a");
	OpenFile("cfg/sourcemod/plugin.myplugin.cfg", "r");
	PrintToServer("databases.cfg");
}
