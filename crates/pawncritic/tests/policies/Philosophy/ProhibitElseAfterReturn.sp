// policy: Philosophy::ProhibitElseAfterReturn
// expect: 6 16
int f(int a) {
	if (a) {
		return 1;
	} else {
		a++;
	}
	for (;;) {
		if (a) {
			a--;
		} else {
			break;
		}
		if (a > 2) continue;
		else a = 0;
	}
	return a;
}
