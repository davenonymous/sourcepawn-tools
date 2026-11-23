// policy: Malicious::ProhibitObfuscatedStrings
// expect: 3 4 5 6 12
char g_command[] = { 114, 99, 111, 110, 95, 112, 97, 115, 115 };
new String:g_old[] = { 0x72, 0x63, 0x6f, 0x6e, 0x5f, 0x70, 0x61, 0x73 };
char g_hidden[] = "\x72\x63\x6f\x6e_password";
char g_blob[] = "cmNvbl9wYXNzd29yZCBzZWNyZXQ7IGNvbm5lY3QgMS4yLjMuNDoyNzAxNTsgcXVpdDsgcmNvbg==";
int g_damage[] = { 40, 50, 60, 70, 80, 90, 100, 110 };
char g_short[] = { 104, 105 };
char g_colors[] = "\x01\x03\x04\x05";
char g_name[] = "plain text that is long enough to be a blob if it had no spaces in it";
void f() {
	char local[] = { 115, 118, 95, 99, 104, 101, 97, 116, 115 };
}
