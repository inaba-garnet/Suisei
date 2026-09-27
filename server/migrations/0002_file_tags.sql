-- ファイルごとの生のタグ（JSON）。変わっていないファイルを読み直さずに、曲とアルバムを組み立て直すため
ALTER TABLE file ADD COLUMN tags TEXT NOT NULL DEFAULT '{}';
