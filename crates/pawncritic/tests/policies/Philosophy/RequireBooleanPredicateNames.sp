// policy: Philosophy::RequireBooleanPredicateNames
// expect: 5 6
bool IsReady() { return true; }
bool HasItems() { return true; }
bool Ready() { return true; }
bool Issue() { return true; }
public bool OnClientConnect(int client, char[] message, int length) { return true; }
int Count() { return 0; }
