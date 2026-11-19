// policy: CodeLayout::ProhibitLongLines
// options: max_length = 40
// expect: 6 7
int g_short;
	int g_indented_but_fine;
		int g_two_tabs_push_this_over_the_limit;
int g_this_one_is_simply_much_longer_than_forty;
