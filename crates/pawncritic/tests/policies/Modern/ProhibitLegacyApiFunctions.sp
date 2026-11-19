// policy: Modern::ProhibitLegacyApiFunctions
// options: allow = ["GetConVarInt"]
// options: extra = ["MyOldFunction = MyNewMethod()"]
// expect: 6 7 10
void f(ConVar cvar, ArrayList list) {
	Handle array = CreateArray();
	PushArrayCell(array, 1);
	list.Push(1);
	int value = GetConVarInt(cvar);
	MyOldFunction();
}
