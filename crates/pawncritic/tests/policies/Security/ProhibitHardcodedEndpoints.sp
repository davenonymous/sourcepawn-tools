// policy: Security::ProhibitHardcodedEndpoints
// expect: 6 7 8 9 10
void f(ConVar cvar) {
	char url[256], host[64];
	cvar.GetString(host, sizeof(host));
	SteamWorks_CreateHTTPRequest(k_EHTTPMethodPOST, "http://1.2.3.4/collect");
	HTTPClient client = new HTTPClient("https://api.example.com");
	SocketConnect(socket, OnConnect, OnReceive, OnDisconnect, "evil.example.com", 80);
	System2HTTPRequest request = new System2HTTPRequest(OnResponse, "http://%s/x" ... "y", host);
	Format(url, sizeof(url), "https://example.com/%s", "x");
	SteamWorks_CreateHTTPRequest(k_EHTTPMethodGET, url);
	Format(url, sizeof(url), "%s/api", host);
	SteamWorks_CreateHTTPRequest(k_EHTTPMethodGET, url);
	SteamWorks_CreateHTTPRequest(k_EHTTPMethodGET, host);
	PrintToChatAll("Join our discord: https://discord.gg/x");
	Updater_AddPlugin("https://example.com/updater.txt");
}
