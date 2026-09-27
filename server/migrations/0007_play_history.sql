-- 再生の履歴。一回の再生を一行で持ち、回数と最後の再生は集計で求める（docs/schema.md）。
-- 同じ曲と同じ時刻の組は一度だけ記録するので、それを主キーにする
CREATE TABLE play_history (
    track_id TEXT NOT NULL REFERENCES track (id) ON DELETE CASCADE,
    -- 再生した時刻（UNIX 時刻のミリ秒）
    played_at BIGINT NOT NULL,
    PRIMARY KEY (track_id, played_at)
);
