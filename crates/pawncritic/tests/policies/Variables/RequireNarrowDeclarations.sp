// policy: Variables::RequireNarrowDeclarations
// expect: 5 14 22 40
void InsideIf(bool flag) {
	PrintToServer("a");
	int value;
	if (flag) {
		value = 1;
		PrintToServer("%d", value);
	}
}

void BufferInLoop() {
	// an array a call fills on every pass
	char name[64];
	for (int client = 1; client <= MaxClients; client++) {
		GetClientName(client, name, sizeof(name));
		PrintToServer("%s", name);
	}
}

void AssignedInLoop() {
	int doubled;
	for (int i = 0; i < 3; i++) {
		doubled = i * 2;
		PrintToServer("%d", doubled);
	}
}

void KeptAcrossPasses(int[] values, int count) {
	int last;
	for (int i = 0; i < count; i++) {
		if (values[i] == last) {
			PrintToServer("repeat");
		}
		last = values[i];
	}
}

void InnerIfInLoop(int count) {
	int found;
	for (int i = 0; i < count; i++) {
		if (i > 1) {
			found = i;
			PrintToServer("%d", found);
		}
	}
}

void SelfReference(int count) {
	int sum = 0;
	for (int i = 0; i < count; i++) {
		sum = sum + i;
	}
}

void ConditionalAssignment(bool flag, int count) {
	int value;
	for (int i = 0; i < count; i++) {
		if (flag) {
			value = i;
		}
		PrintToServer("%d", value);
	}
}

void InConditionToo(bool flag) {
	int value;
	if (flag && value > 0) {
		value = 1;
	}
}

void UsedAfter(bool flag) {
	int value;
	if (flag) {
		value = 1;
	}
	PrintToServer("%d", value);
}

void Unbraced(bool flag) {
	int value;
	if (flag) PrintToServer("%d", value);
}

void CallInitializer(bool flag) {
	float now = GetGameTime();
	if (flag) {
		PrintToServer("%f", now);
	}
}

void ComparedWithEarlierPasses(int count) {
	// an array a call only reads: it keeps the newest name of the earlier passes
	char newest[64];
	int found = -1;
	for (int i = 0; i < count; i++) {
		char name[64];
		GetName(i, name, sizeof(name));
		if (found == -1 || strcmp(name, newest) > 0) {
			found = i;
			strcopy(newest, sizeof(newest), name);
		}
	}
	PrintToServer("%d", found);
}

void AppendedInLoop(int count) {
	// a call that gets the array besides its buffer reads it too
	char line[64];
	for (int i = 0; i < count; i++) {
		Format(line, sizeof(line), "%s%d", line, i);
		PrintToServer("%s", line);
	}
}
