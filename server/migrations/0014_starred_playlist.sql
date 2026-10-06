-- お気に入りのプレイリスト（docs/schema.md の「お気に入りのプレイリスト」）。一行だけ持つ。
-- 曲の並びは持たず、お気に入りの曲の指紋と、指紋が変わったのに気づいた日時（UNIX 時刻のミリ秒）を持つ
CREATE TABLE starred_playlist (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    created_at BIGINT NOT NULL,
    changed_at BIGINT NOT NULL,
    fingerprint TEXT NOT NULL
);

INSERT INTO starred_playlist (id, created_at, changed_at, fingerprint)
VALUES (1, CAST(strftime('%s', 'now') AS INTEGER) * 1000,
        CAST(strftime('%s', 'now') AS INTEGER) * 1000, '');
