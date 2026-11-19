// policy: Modern::ProhibitFunctagAndFuncenum
// expect: 3 4
functag public OldCallback(client);
funcenum OldCallbacks { public(client), public() };
typedef NewCallback = function void (int client);
