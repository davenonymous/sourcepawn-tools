// policy: Bugs::RequireInclusiveMaxClients
// expect: 4 6
void f() {
	for (int i = 1; i < MaxClients; i++) {
	}
	for (int i = 1; MaxClients > i; i++) {
	}
	for (int client = 1; client <= MaxClients; client++) {
	}
	for (int i = 0; i < 10; i++) {
	}
}
