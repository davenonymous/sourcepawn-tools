// policy: Modern::ProhibitDeprecatedFunctions
// expect: 5 6 9
void f(int client) {
	char auth[32];
	GetClientAuthString(client, auth, sizeof(auth));
	float x = FloatMul(1.0, 2.0);
	GetClientAuthId(client, AuthId_Steam2, auth, sizeof(auth));
}
public void OnServerCfg() {
}
