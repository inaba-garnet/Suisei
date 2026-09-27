-- お気に入りにした日時（UNIX 時刻のミリ秒）と評価（1〜5）。スキャンは書かないので再スキャンで消えない（docs/schema.md）
ALTER TABLE artist ADD COLUMN starred_at BIGINT;
ALTER TABLE artist ADD COLUMN rating INTEGER;
ALTER TABLE album ADD COLUMN starred_at BIGINT;
ALTER TABLE album ADD COLUMN rating INTEGER;
ALTER TABLE track ADD COLUMN starred_at BIGINT;
ALTER TABLE track ADD COLUMN rating INTEGER;
