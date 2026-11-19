// policy: Philosophy::ProhibitHiddenGlobalMutation
// expect: 6
int g_count;
int g_values[4];

void Reset() {
	g_count = 0;
	g_values[1]++;
}
int Next() {
	return ++g_count;
}
public void OnMapStart() {
	g_count = 0;
}
void Shadowed() {
	int g_count = 1;
	g_count++;
}
