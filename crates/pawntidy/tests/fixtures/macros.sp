#define LOG(%1) if (g_debug) LogMessage(%1)
#define TWICE a(); b()

void Macros()
{
	if (a) LOG("x"); else c();
	if (b) TWICE;
	if (c) DEBUG_PRINT("from an include");
	if (d) PrintToServer("braced");
}
