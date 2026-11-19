// policy: Philosophy::ProhibitEmptyBlocks
// expect: 4 6 10
void f() {
	if (a) {
	}
	for (;;) {}
	while (b) {
		// waiting is the point
	}
	switch (c) { case 1: { } }
}
void EmptyFunctionsAreFine() {
}
