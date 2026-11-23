// policy: Security::ProhibitCodeFileWrites
// expect: 5 6 7 7 8 9 12
void f() {
	char path[PLATFORM_MAX_PATH];
	DeleteFile("addons/sourcemod/plugins/basebans.smx");
	OpenFile("addons/sourcemod/plugins/evil.smx", "wb");
	RenameFile("addons/metamod/x.vdf", "addons/metamod/y.txt");
	BuildPath(Path_SM, path, sizeof(path), "extensions/evil.ext.so");
	SteamWorks_WriteHTTPResponseBodyToFile(request, "bin/server.dll");
	OpenFile(path, "w");
	OpenFile("addons/sourcemod/plugins/x.smx", "rb");
	BuildPath(Path_SM, path, sizeof(path), "plugins/disabled/%s.smx", "x");
	DeleteFile(path);
	OpenFile("addons/sourcemod/data/x.txt", "w");
	DeleteFile("addons/sourcemod/logs/x.log");
}
