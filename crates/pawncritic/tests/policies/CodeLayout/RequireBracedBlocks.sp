// policy: CodeLayout::RequireBracedBlocks
// expect: 6 8 8 9 10 11 14
void f() {
	if (a) {
		b();
	} else if (c) d();
	else {
		if (e) f(); else g();
		for (;;) h();
		while (i) j();
		do k(); while (l);
	}
	switch (m) {
		case 1: n();
		default: {
		}
	}
}
