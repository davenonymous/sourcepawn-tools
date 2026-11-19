// policy: Complexity::ProhibitLongFunctions
// options: max_lines = 4
// expect: 4 21 27 46
void TooLong() {
	int a = 1;
	int b = 2;
	PrintToServer("%d %d", a, b);
}

// Comments and blank lines do not count.
void JustRight() {
	// a comment

	int a = 1;
	/* another
	   comment */
	PrintToServer("%d", a);
}

methodmap Thing < Handle {
	public void Method() {
		int a = 1;
		int b = 2;
		PrintToServer("%d %d", a, b);
	}
	property int Value {
		public get() {
			int a = 1;
			int b = 2;
			return a + b;
		}
	}
	public native void Native();
}

// A list of optional natives is not logic to split.
public void __pl_thing_SetNTVOptional() {
	MarkNativeAsOptional("Thing_One");
	MarkNativeAsOptional("Thing_Two");
	// a comment
	MarkNativeAsOptional("Thing_Three");
	MarkNativeAsOptional("Thing_Four");
}

// Anything else among them makes it a function like any other.
public void __pl_other_SetNTVOptional() {
	MarkNativeAsOptional("Other_One");
	MarkNativeAsOptional("Other_Two");
	PrintToServer("loaded");
	MarkNativeAsOptional("Other_Three");
}
