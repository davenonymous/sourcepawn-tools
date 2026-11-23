// policy: CodeLayout::ProhibitMixedIndentation
// expect: 6
void f() {
    int a = 1;
    Call(a,
	     a);
    int b = 2;
	int c = 3;
}
