// policy: Variables::RequireLateDeclarations
// expect: 6 13 26 61
void Grouped(int client) {
	// `name` follows only another declaration; `auth` follows a call.
	char name[64];
	char auth[32];
	GetClientName(client, name, sizeof(name));
	GetClientAuthId(client, AuthId_Steam2, auth, sizeof(auth));
}

void Constant() {
	// a constant initializer may move, a call may not
	int count = 0;
	float now = GetGameTime();
	PrintToServer("start");
	count++;
	PrintToServer("%d %f", count, now);
}

void RightThere(int client) {
	char name[64];
	GetClientName(client, name, sizeof(name));
}

void BeforeLoop() {
	int total;
	PrintToServer("counting");
	for (int i = 0; i < 3; i++) {
		total += i;
	}
	PrintToServer("%d", total);
}

void Unused() {
	int never;
	PrintToServer("never used: spcomp reports that");
}

void Shadowed() {
	int value;
	PrintToServer("a");
	{
		int value = 2;
		PrintToServer("%d", value);
	}
	value = 1;
}

void OnlyInsideIf(bool flag) {
	// left to Variables::RequireNarrowDeclarations
	int value;
	PrintToServer("a");
	if (flag) {
		value = 1;
		PrintToServer("%d", value);
	}
}

// `thing.Field` is a member, not the variable, so the first use is two lines down.
void Members(Thing thing) {
	int Field;
	thing.Field = 1;
	PrintToServer("%d", Field);
}
