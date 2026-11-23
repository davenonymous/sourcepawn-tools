// policy: Security::ProhibitShellExecution
// expect: 4 5
void f() {
	System2_ExecuteThreaded(OnDone, "ls -la");
	System2_ExecuteFormatted(output, sizeof(output), "rm -rf %s", "x");
	ServerCommand("echo System2_Execute");
	helper.System2_Execute("x");
}
