// policy: Philosophy::ProhibitBareHandle
// options: extra = ["MyExt_CreateThing=Thing"]
// expect: 4 5 6 11 12
Handle g_list = null;
Handle g_cvar;
new Handle:g_old = CreateTrie();
Handle g_timer;
Handle g_hud;
public void OnPluginStart() {
	g_list = CreateArray();
	Handle menu = new Menu(Handler);
	Handle thing = MyExt_CreateThing();
	Handle unknown = SomethingElse();
	g_cvar = FindConVar("sv_cheats");
	g_timer = CreateTimer(1.0, Tick);
	g_hud = CreateHudSynchronizer();
}
void Other() {
	Handle menu;
	menu = null;
}
void Fill() {
	ArrayList list;
	list = CreateArray();
	delete list;
}
methodmap Thing < Handle {
	public ~Thing() {
		Handle list;
		delete list;
	}
}
