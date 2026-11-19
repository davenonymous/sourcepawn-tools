// policy: CodeLayout::RequireSameLineBraces
// expect: 8 10 17 21
void Good() {
	if (a) {
	}
}
void Bad()
{
	if (a) // comment
	{
	}
	{
		int scoped;
	}
}
enum Color
{
	Red
}
methodmap Thing < Handle
{
}
