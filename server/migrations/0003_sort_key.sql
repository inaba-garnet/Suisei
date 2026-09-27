-- 並べ替えのキー。アプリで計算し、DB の照合順序に頼らない（docs/server.md）。次のスキャンで埋まる
ALTER TABLE artist ADD COLUMN sort_key TEXT NOT NULL DEFAULT '';
ALTER TABLE album ADD COLUMN sort_key TEXT NOT NULL DEFAULT '';
ALTER TABLE track ADD COLUMN sort_key TEXT NOT NULL DEFAULT '';
