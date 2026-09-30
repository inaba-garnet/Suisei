-- 作曲、作詞、編曲（docs/schema.md の「作曲、作詞、編曲」）。role は composer、lyricist、arranger
CREATE TABLE track_contributor (
    track_id TEXT NOT NULL REFERENCES track (id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    position INTEGER NOT NULL,
    artist_id TEXT NOT NULL REFERENCES artist (id),
    credited_name TEXT NOT NULL,
    PRIMARY KEY (track_id, role, position)
);

CREATE INDEX track_contributor_artist ON track_contributor (artist_id);

-- COMPOSER をそのまま表示用に持つ（displayComposer）
ALTER TABLE track ADD COLUMN display_composer TEXT;
