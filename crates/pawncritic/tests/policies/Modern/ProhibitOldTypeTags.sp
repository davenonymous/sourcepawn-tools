// policy: Modern::ProhibitOldTypeTags
// expect: 3 3 4 5
public Action:Timer(Handle:timer) {
	new String:name[32];
	new bool:flag = _:true;
	int value = view_as<int>(1.0);
}
