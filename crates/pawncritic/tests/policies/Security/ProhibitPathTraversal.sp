// policy: Security::ProhibitPathTraversal
// expect: 5 6 7 8 11 14
void f() {
	char path[PLATFORM_MAX_PATH];
	OpenFile("../server.log", "r");
	DeleteFile("maps/../../srcds_run");
	BuildPath(Path_SM, path, sizeof(path), "../../../etc/passwd");
	RenameFile("ok.txt", "cfg\\..\\..\\x.txt");
	BuildPath(Path_SM, path, sizeof(path), "../../cfg/sourcemod/x.cfg");
	OpenFile(path, "r");
	Format(path, sizeof(path), "logs/../../%s", "x");
	OpenFile(path, "a");
	Format(path, sizeof(path), "%s/../../x", "a");
	FileExists("x/../../y", true);
	OpenFile("cfg/../maps/x.txt", "r");
	PrintToServer("../../not/a/path");
}
