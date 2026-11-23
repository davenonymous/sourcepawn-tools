// policy: Malicious::ProhibitClientSlowhack
// expect: 5 6 7 8 13
void f(int client) {
	char command[128];
	ClientCommand(client, "bind w \"say hi\"");
	ClientCommand(client, "play buttons/blip1.wav; connect 1.2.3.4:27015");
	ClientCommand(client, "  UNBINDALL");
	Format(command, sizeof(command), "redirect %s", "1.2.3.4");
	ClientCommand(client, command);
	ClientCommand(client, "play buttons/blip1.wav");
	FakeClientCommand(client, "bind w kill");
	ClientCommand(client, "r_screenoverlay 0");
	ClientCommand(client, "motdfile cfg/autoexec.cfg\nmotd_write echo hi");
}
