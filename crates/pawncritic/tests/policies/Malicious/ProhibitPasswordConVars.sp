// policy: Malicious::ProhibitPasswordConVars
// expect: 6 7 8 9
void f() {
	char password[64];
	ConVar hostname = FindConVar("hostname");
	ConVar rcon = FindConVar("rcon_password");
	ServerCommand("sm_cvar RCON_PASSWORD %s", password);
	ServerCommand("sv_rcon_banpenalty 0");
	ServerCommand("rcon" ... "_password x");
	ServerCommand("sv_password secret");
	PrintToServer("my_rcon_password_helper");
}
