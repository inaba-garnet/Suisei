-- OpenSubsonic の isCompilation と、Subsonic の曲の created に使う。次のスキャンで埋まる
ALTER TABLE album ADD COLUMN compilation BOOLEAN NOT NULL DEFAULT FALSE;
-- 曲を初めて見つけた日時（UNIX 時刻のミリ秒）。0 はまだ記録していない
ALTER TABLE track ADD COLUMN created_at BIGINT NOT NULL DEFAULT 0;
