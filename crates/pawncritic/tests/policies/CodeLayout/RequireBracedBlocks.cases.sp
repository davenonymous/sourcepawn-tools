// policy: CodeLayout::RequireBracedBlocks
// options: check_switch_cases = false
// expect:
void f() {
	switch (m) {
		case 1: n();
	}
}
