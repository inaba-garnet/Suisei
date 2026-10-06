-- Spotify 連携（docs/spotify.md）。日時は UNIX 時刻のミリ秒

-- 接続は一つだけ持つ。アクセストークンはメモリにだけ持つ
CREATE TABLE spotify_account (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    refresh_token TEXT NOT NULL,
    connected_at BIGINT NOT NULL,
    -- 最後に Spotify からお気に入りの一覧を読み始めた日時
    fetched_at BIGINT
);

-- Spotify のお気に入りの曲と、対応するローカルの曲。
-- スキャンで曲が増えたときに、Spotify を呼ばずに対応を付け直すため、曲の情報も持つ
CREATE TABLE spotify_track (
    spotify_id TEXT PRIMARY KEY,
    isrc TEXT,
    title TEXT NOT NULL,
    -- 一人ずつの名前の JSON の配列
    artists TEXT NOT NULL,
    album TEXT NOT NULL,
    disc_number INTEGER,
    track_number INTEGER,
    duration_ms BIGINT NOT NULL,
    -- Spotify でお気に入りにした日時
    added_at BIGINT NOT NULL,
    -- 曲が消えたら未対応に戻す
    track_id TEXT REFERENCES track (id) ON DELETE SET NULL,
    -- isrc、match_key、fuzzy、manual、ignored（手動で外した）
    match_method TEXT
);

CREATE INDEX spotify_track_track ON spotify_track (track_id);
