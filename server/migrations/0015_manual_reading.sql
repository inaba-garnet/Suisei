-- 読みの手動設定（docs/schema.md の「日本語の並べ替え」）。
-- sort_name と sort_name_source は、手動の読みがあればそれを、なければ自動の読みを持つ。
-- auto_ はスキャンで得た自動の読み。手動の読みを外したとき、スキャンを待たずに戻すため。
-- manual_ は手動の読みと、設定したときの名前。スキャンは書かず、名前が同じときだけ使う。
ALTER TABLE artist ADD COLUMN auto_sort_name TEXT;
ALTER TABLE artist ADD COLUMN auto_sort_name_source TEXT;
ALTER TABLE artist ADD COLUMN manual_sort_name TEXT;
ALTER TABLE artist ADD COLUMN manual_name TEXT;
ALTER TABLE album ADD COLUMN auto_sort_name TEXT;
ALTER TABLE album ADD COLUMN auto_sort_name_source TEXT;
ALTER TABLE album ADD COLUMN manual_sort_name TEXT;
ALTER TABLE album ADD COLUMN manual_name TEXT;
ALTER TABLE track ADD COLUMN auto_sort_name TEXT;
ALTER TABLE track ADD COLUMN auto_sort_name_source TEXT;
ALTER TABLE track ADD COLUMN manual_sort_name TEXT;
ALTER TABLE track ADD COLUMN manual_name TEXT;

UPDATE artist SET auto_sort_name = sort_name, auto_sort_name_source = sort_name_source;
UPDATE album SET auto_sort_name = sort_name, auto_sort_name_source = sort_name_source;
UPDATE track SET auto_sort_name = sort_name, auto_sort_name_source = sort_name_source;
