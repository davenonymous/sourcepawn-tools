// policy: Malicious::ProhibitVguiCommandBypass
// expect: 8 18 28
void Show(int client) {
	KeyValues kv = new KeyValues("data");
	kv.SetString("title", "Rules");
	kv.SetString("type", "2");
	kv.SetString("msg", "http://example.com");
	kv.SetString("cmd", "bind w kill");
	ShowVGUIPanel(client, "info", kv);
	delete kv;
}

void ShowOld(int client, const char[] command) {
	Handle kv = CreateKeyValues("data");
	KvSetString(kv, "cmd", "closed_htmlpage");
	KvSetNum(kv, "cmd", 5);
	KvSetNum(kv, "cmd", PANEL_CMD_CLOSED_HTMLPAGE);
	KvSetString(kv, "cmd", command);
	ShowVGUIPanel(client, "info", kv);
	CloseHandle(kv);
}

void ShowMessage(int client) {
	Handle bf = StartMessageOne("VGUIMenu", client);
	BfWriteString(bf, "info");
	BfWriteByte(bf, 1);
	BfWriteByte(bf, 1);
	BfWriteString(bf, "cmd");
	BfWriteString(bf, "connect 1.2.3.4");
	EndMessage();
}

void NoPanel(KeyValues kv) {
	kv.SetString("cmd", "anything");
}
