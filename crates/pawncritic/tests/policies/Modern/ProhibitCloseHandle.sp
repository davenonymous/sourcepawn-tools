// policy: Modern::ProhibitCloseHandle
// expect: 4 6
void f(Handle h) {
	CloseHandle(h);
	delete h;
	if (CloseHandle(h)) {
	}
	CloseHandle(h); // no critic (CloseHandle)
}
