-- プレイリストと、その曲の並び（docs/schema.md）。日時は UNIX 時刻のミリ秒
CREATE TABLE playlist (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    comment TEXT,
    public BOOLEAN NOT NULL,
    created_at BIGINT NOT NULL,
    changed_at BIGINT NOT NULL
);

-- 同じ曲を何度でも入れられるよう、位置を主キーに含める。曲が消えたら並びからも除く
CREATE TABLE playlist_entry (
    playlist_id TEXT NOT NULL REFERENCES playlist (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    track_id TEXT NOT NULL REFERENCES track (id) ON DELETE CASCADE,
    PRIMARY KEY (playlist_id, position)
);

CREATE INDEX playlist_entry_track ON playlist_entry (track_id);
