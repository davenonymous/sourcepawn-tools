// policy: Variables::RequireLateDeclarations
// options: min_statements = 2
// expect: 11
void OneStatement() {
	int a;
	PrintToServer("one");
	a = 1;
}

void TwoStatements() {
	int b;
	PrintToServer("one");
	PrintToServer("two");
	b = 1;
}
