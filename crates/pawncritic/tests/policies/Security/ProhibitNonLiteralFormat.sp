// policy: Security::ProhibitNonLiteralFormat
// expect: 6 7 8 9 10
#define PREFIX "[X] "
void f(int client, const char[] message) {
	char buffer[128];
	PrintToChat(client, message);
	PrintToChatAll(buffer);
	Format(buffer, sizeof(buffer), message);
	ETF2L_PrintToChat(client, message);
	CPrintToChatAll(PREFIX ... message);
	PrintToChat(client, "%s", message);
	PrintToChat(client, message, client);
	PrintToChat(client, PREFIX ... "hello");
	PrintToChat(client, PREFIX);
	ReplyToCommand(client, "[X] %t", "Phrase");
	PrintToServer("plain");
}
