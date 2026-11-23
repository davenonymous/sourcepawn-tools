// policy: CodeLayout::ProhibitMixedIndentation
// expect:
void f() {
    int a = 1;
    Call(a,
         a);
    if (a) {
        a = 2;
    }
}
int g_total = First +
              Second;
