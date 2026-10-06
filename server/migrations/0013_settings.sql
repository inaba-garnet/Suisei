-- Web から変える設定（docs/server.md の「設定」）。一行だけ持つ。間隔は秒
CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    scan_interval_secs BIGINT NOT NULL,
    backup_interval_secs BIGINT NOT NULL,
    backup_keep INTEGER NOT NULL,
    split_characters BOOLEAN NOT NULL,
    spotify_client_id TEXT
);

-- 環境変数で渡していたときの既定値
INSERT INTO settings (id, scan_interval_secs, backup_interval_secs, backup_keep, split_characters)
VALUES (1, 3600, 86400, 7, TRUE);
