// policy: CodeLayout::ProhibitMixedIndentation
// expect: 7
void f() {
	int a = 1;
	Call(a,
	     a);
    int spaces = 2;
	 	int mixed = 3;
    // a comment indented with spaces
	/*
	 * aligned comment lines are fine
	 */
	Call(a,
			a);
}
