-- Web クライアントのログインのセッション（docs/schema.md）。ID はハッシュだけを持つ。日時は UNIX 時刻のミリ秒
CREATE TABLE session (
    id_hash TEXT PRIMARY KEY,
    created_at BIGINT NOT NULL,
    last_used_at BIGINT NOT NULL
);
