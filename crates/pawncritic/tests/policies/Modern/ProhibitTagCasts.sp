// policy: Modern::ProhibitTagCasts
// expect: 4 5
void f() {
	int a = _:2.0;
	float b = Float:a;
	int c = view_as<int>(b);
}
