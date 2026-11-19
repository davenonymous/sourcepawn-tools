// policy: Complexity::ProhibitLongFiles
// options: max_lines = 5
// expect:

// Five code lines are allowed.
void f() {
	int a = 1;
	PrintToServer("%d", a);
}
#define X 1
