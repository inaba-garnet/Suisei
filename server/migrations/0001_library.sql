-- 閲覧と再生に使うライブラリの表。
-- SQLite と PostgreSQL に共通する書き方に限る（docs/server.md）。
-- ID は種類の接頭辞と 16 進 8 文字の乱数（docs/schema.md）。日時は UNIX 時刻のミリ秒。

CREATE TABLE music_folder (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE
);

CREATE TABLE artist (
    id TEXT PRIMARY KEY,
    match_key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    -- 読み（カタカナ）と、どの規則で得たか
    sort_name TEXT,
    sort_name_source TEXT
);

CREATE TABLE album (
    id TEXT PRIMARY KEY,
    match_key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    display_artist TEXT NOT NULL,
    sort_name TEXT,
    sort_name_source TEXT,
    year INTEGER,
    -- 初回検出の日時
    created_at BIGINT NOT NULL
);

CREATE TABLE track (
    id TEXT PRIMARY KEY,
    match_key TEXT NOT NULL UNIQUE,
    album_id TEXT NOT NULL REFERENCES album (id),
    title TEXT NOT NULL,
    display_artist TEXT NOT NULL,
    sort_name TEXT,
    sort_name_source TEXT,
    disc_number INTEGER,
    track_number INTEGER,
    year INTEGER,
    -- 配信に使うファイル。file と互いに参照するので外部キーは付けない
    primary_file_id TEXT
);

CREATE INDEX track_album ON track (album_id);

CREATE TABLE file (
    id TEXT PRIMARY KEY,
    music_folder_id INTEGER NOT NULL REFERENCES music_folder (id),
    -- 音楽フォルダからの相対パス
    path TEXT NOT NULL,
    size BIGINT NOT NULL,
    mtime BIGINT NOT NULL,
    track_id TEXT NOT NULL REFERENCES track (id),
    suffix TEXT NOT NULL,
    content_type TEXT NOT NULL,
    duration_ms BIGINT NOT NULL,
    bit_rate INTEGER,
    sample_rate INTEGER,
    channels INTEGER,
    bit_depth INTEGER,
    lossless BOOLEAN NOT NULL,
    UNIQUE (music_folder_id, path)
);

CREATE INDEX file_track ON file (track_id);

-- credited_name と credited_sort は、そのファイルのタグの表記。表示名と読みを多数派で決めるのに使う
CREATE TABLE track_artist (
    track_id TEXT NOT NULL REFERENCES track (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    artist_id TEXT NOT NULL REFERENCES artist (id),
    credited_name TEXT NOT NULL,
    credited_sort TEXT,
    PRIMARY KEY (track_id, position)
);

CREATE INDEX track_artist_artist ON track_artist (artist_id);

CREATE TABLE album_artist (
    album_id TEXT NOT NULL REFERENCES album (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    artist_id TEXT NOT NULL REFERENCES artist (id),
    credited_name TEXT NOT NULL,
    credited_sort TEXT,
    PRIMARY KEY (album_id, position)
);

CREATE INDEX album_artist_artist ON album_artist (artist_id);

CREATE TABLE track_genre (
    track_id TEXT NOT NULL REFERENCES track (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    genre TEXT NOT NULL,
    PRIMARY KEY (track_id, position)
);

CREATE INDEX track_genre_genre ON track_genre (genre);

-- 手動でマージして消えた側の ID
CREATE TABLE id_alias (
    old_id TEXT PRIMARY KEY,
    new_id TEXT NOT NULL
);
