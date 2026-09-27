# 検証の方針

## 検証用サーバー
- Fly.io の Sprites で動かす。休止中は計算資源の料金がかからず、リクエストで自動的に起き、ディスク上の SQLite が公式に保証されているため。
- Claude は環境変数 `SPRITES_TOKEN` で Sprites の API を直接操作し、デプロイとログ確認を自分で行う。
- URL は公開にし、Subsonic の認証で守る。Symfonium は Fly.io の組織認証を通れないため。
- Sprite は Docker ではなく素の Linux で動く。本番用の Docker イメージは CI でビルドできることだけ確かめる。

## 動作確認
- 動作確認の基準クライアントは Symfonium。
- Symfonium が実際に送るリクエストを記録し、テストで再生する。
- 応答の妥当性は Navidrome（`demo.navidrome.org`）の応答と構造、必須項目を比べて確かめる。取得した応答はファイルに保存して使う。
