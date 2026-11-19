// policy: Modern::ProhibitOldDeclarations
// expect: 4 5 6 8 9
int g_modern = 1;
new g_old = 1;
new Float:g_tagged;
public OnPluginStart() {
	int modern;
	decl String:buffer[64];
	for (new i = 0; i < 3; i++) {
	}
}
public void Fine() {
}
