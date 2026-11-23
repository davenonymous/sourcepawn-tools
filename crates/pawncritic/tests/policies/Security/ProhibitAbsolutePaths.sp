// policy: Security::ProhibitAbsolutePaths
// expect: 5 6 7 8
void f() {
	char path[PLATFORM_MAX_PATH];
	OpenFile("/etc/passwd", "r");
	DeleteFile("C:\\Windows\\system.ini");
	FileToKeyValues(kv, "\\\\host\\share\\x.cfg");
	Format(path, sizeof(path), "/home/%s/.ssh/id_rsa", "srcds");
	OpenFile(path, "r");
	BuildPath(Path_SM, path, sizeof(path), "/configs/x.cfg");
	OpenFile("cfg/x.cfg", "r");
	PrintToServer("/etc/passwd");
}
