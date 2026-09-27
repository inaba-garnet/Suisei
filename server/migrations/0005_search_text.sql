-- 検索で照らす文字列。名前と読みを正規化してつなぐ。アプリで作り、次のスキャンで埋まる
ALTER TABLE artist ADD COLUMN search_text TEXT NOT NULL DEFAULT '';
ALTER TABLE album ADD COLUMN search_text TEXT NOT NULL DEFAULT '';
ALTER TABLE track ADD COLUMN search_text TEXT NOT NULL DEFAULT '';
