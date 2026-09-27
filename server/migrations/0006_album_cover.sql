-- カバーアートの元。音楽フォルダからの相対パスで、画像ならその画像、音声なら埋め込みの画像を使う。なければ NULL
ALTER TABLE album ADD COLUMN cover_path TEXT;
