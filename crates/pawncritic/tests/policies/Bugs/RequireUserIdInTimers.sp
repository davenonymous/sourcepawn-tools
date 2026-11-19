// policy: Bugs::RequireUserIdInTimers
// expect: 4 5
void f(int client) {
	CreateTimer(1.0, Timer_Run, client);
	RequestFrame(Frame_Run, client);
	CreateTimer(1.0, Timer_Run, GetClientUserId(client));
	CreateTimer(1.0, Timer_Run, count);
	CreateTimer(1.0, Timer_Run);
}
