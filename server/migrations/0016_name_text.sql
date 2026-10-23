-- 検索で一致の度合いを見るための、名前と読みだけを正規化した文字列（docs/schema.md の「検索」）。
-- アーティストは search_text が名前と読みだけなので持たない。アプリで作り、次のスキャンで埋まる
ALTER TABLE album ADD COLUMN name_text TEXT NOT NULL DEFAULT '';
ALTER TABLE track ADD COLUMN name_text TEXT NOT NULL DEFAULT '';
