// policy: Philosophy::ProhibitDeepNesting
// options: max_depth = 2
// expect: 7
void f() {
	if (a) {
		for (;;) {
			while (b) {
				if (c) {
				}
			}
		}
	} else if (d) {
		if (e) {
		} else if (f) {
		}
	}
}
