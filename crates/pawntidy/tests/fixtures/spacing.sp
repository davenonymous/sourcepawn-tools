#pragma semicolon 1
#pragma newdecls required

int g_Values[3]={1,2,3};
char g_Name[ 64 ];

public void Spacing(int &ref,const char[] text,any ...)
{
	int a=-1,b=+2;
	a=a+b*2-(b%3);
	a+=view_as<int>( 1.0 );
	a = !a ? ~b : b<<2;
	a++;--b;
	bool same = a==b&&a!=0||b>=1;
	Format(text,sizeof (text),"%d",a);
	Call(.name=1,.other = 2);
	g_Values[0]=g_Name{1};
	char packed[] = !"packed";
	ref = Float:a + _:b;
	SomeMap.Method( ).Field=1;
	return ;
}

methodmap Thing<Handle
{
	public Thing() { return view_as<Thing>(0); }
	public ~Thing() {}
	property int Size
	{
		public get() { return 1; }
	}
}
