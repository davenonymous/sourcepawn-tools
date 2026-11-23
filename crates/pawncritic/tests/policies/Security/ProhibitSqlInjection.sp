// policy: Security::ProhibitSqlInjection
// expect: 8 19 19
Database g_db;
public void OnClientPostAdminCheck(int client) {
	char name[64], safe[129], query[512];
	GetClientName(client, name, sizeof(name));
	g_db.Escape(name, safe, sizeof(safe));
	Format(query, sizeof(query), "INSERT INTO players (name) VALUES ('%s')", name);
	Format(query, sizeof(query), "INSERT INTO players (name) VALUES ('%s')", safe);
	g_db.Format(query, sizeof(query), "INSERT INTO players (name) VALUES ('%s')", name);
	Format(query, sizeof(query), "SELECT * FROM players WHERE id = %d", client);
	Format(query, sizeof(query), "Hello %s", name);
}

public void OnClientSayCommand_Post(int client, const char[] command, const char[] sArgs) {
	char query[512], escaped[512], text[256];
	strcopy(text, sizeof(text), sArgs);
	FormatEx(query, sizeof(query), "INSERT INTO chat VALUES (%d, '%T', '%s', '%s')", client, "Phrase", client,
		text, sArgs);
	SQL_EscapeString(g_db, sArgs, escaped, sizeof(escaped));
	Format(query, sizeof(query), "update chat set text = '%s' where 100%% = 100%%", escaped);
}
