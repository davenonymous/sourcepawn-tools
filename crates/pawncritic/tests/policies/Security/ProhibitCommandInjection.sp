// policy: Security::ProhibitCommandInjection
// expect: 7 9 10 18 26
public Action Command_Kick(int client, int args) {
	char target[64], reason[128], command[256];
	GetCmdArg(1, target, sizeof(target));
	GetCmdArgString(reason, sizeof(reason));
	ServerCommand("sm_kick %s", target);
	Format(command, sizeof(command), "sm_ban %s 0", target);
	ServerCommand(command);
	FakeClientCommand(client, "say %s", reason);
	ServerCommand("sm_kick #%d", GetClientUserId(client));
	return Plugin_Handled;
}

public void OnClientSayCommand_Post(int client, const char[] command, const char[] sArgs) {
	char copy[256];
	strcopy(copy, sizeof(copy), sArgs);
	ClientCommand(client, copy);
	ServerCommand("say %s", command);
}

void Rename(int client) {
	char name[64];
	ServerCommand("sm_rename %s", name);
	GetClientName(client, name, sizeof(name));
	ServerCommandEx(name, sizeof(name), "sm_rename #%d %s", 1, name);
}
