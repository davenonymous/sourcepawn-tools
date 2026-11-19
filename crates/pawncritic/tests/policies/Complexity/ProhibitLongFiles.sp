// policy: Complexity::ProhibitLongFiles
// options: max_lines = 5
// expect: 1

// These six lines hold code; comments and blank lines do not count.
void f() {
	int a = 1;

	PrintToServer("%d", a);
}
#define X 1
#define Y 2
