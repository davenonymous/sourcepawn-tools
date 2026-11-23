// policy: Security::ProhibitServerControlCommands
// expect: 5 6 7 8 9
void f() {
	char command[64];
	ServerCommand("sm plugins unload basebans");
	ServerCommand("echo hi; QUIT");
	InsertServerCommand("meta   load addons/evil");
	ServerCommandEx(command, sizeof(command), "sm exts load evil.ext");
	Format(command, sizeof(command), "_restart");
	ServerCommand(command);
	ServerCommand("changelevel cp_badlands");
	ServerCommand("exec server.cfg");
	ServerCommand("sm_plugins_helper load");
	ClientCommand(client, "quit");
}
