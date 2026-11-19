public void OnPluginStart()
{
	g_Cvar = CreateConVar("sm_example_enabled", "1", "Whether the example runs at all; 0 stops it", _, true, 0.0, true, 1.0);
	if (IsClientInGame(client) && !IsFakeClient(client) && GetClientTeam(client) == TEAM_RED && IsPlayerAlive(client) && IsClientReady(client))
	{
		PrintToChat(client, "%s", "short");
	}
	int total = FirstValueWithALongName + SecondValueWithALongName + ThirdValueWithALongName + FourthValue + FifthValueAsWell;
	PrintToServer("a string so long that it can never fit into the line however hard pawntidy tries to make it fit in");
}

public Action OnPlayerRunCmd(int client, int &buttons, int &impulse, float vel[3], float angles[3], int &weapon, int &subtype)
{
	return Plugin_Continue;
}

void UserBroken()
{
	Call(first,
	second,
	third);
	Call(
		first,
		second
	);
}
