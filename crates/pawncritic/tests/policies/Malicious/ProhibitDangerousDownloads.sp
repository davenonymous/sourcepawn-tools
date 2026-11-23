// policy: Malicious::ProhibitDangerousDownloads
// expect: 5 6 7 8 9
public void OnMapStart() {
	char path[PLATFORM_MAX_PATH];
	AddFileToDownloadsTable("cfg/autoexec.cfg");
	AddFileToDownloadsTable("materials/x/../../config.CFG");
	AddFileToDownloadsTable("bin\\client.dll");
	AddFileToDownloadsTable("./cfg/game.txt");
	Format(path, sizeof(path), "download/%s.exe", "setup");
	AddFileToDownloadsTable(path);
	AddFileToDownloadsTable("materials/models/player/skin.vtf");
	AddFileToDownloadsTable("sound/music/song.mp3");
}
