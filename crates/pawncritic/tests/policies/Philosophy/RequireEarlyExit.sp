// policy: Philosophy::RequireEarlyExit
// expect: 4 12 31
void Wrapped(int client) {
	if (IsClientInGame(client)) {
		a();
		b();
		c();
	}
}
void Loop() {
	for (int i = 1; i <= MaxClients; i++) {
		if (IsClientInGame(i)) {
			a();
			b();
			c();
		}
	}
}
void Fine(int client) {
	if (!IsClientInGame(client)) {
		return;
	}
	a();
}
void Small(int client) {
	if (IsClientInGame(client)) {
		a();
	}
}
stock float operator%(float value, float divisor) {
	if (divisor != 0.0) {
		a();
		b();
		c();
	}
}
