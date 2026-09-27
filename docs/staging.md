# 検証用サーバーの運用

検証用サーバーは Sprite `suisei-staging`（`https://suisei-staging-b3k73.sprites.app`）。
操作は Sprites の API（`https://api.sprites.dev/v1/sprites/suisei-staging`）に `Authorization: Bearer $SPRITES_TOKEN` を付けて行う。

## 配置
| パス | 中身 |
| --- | --- |
| `/home/sprite/suisei/suisei` | musl の静的バイナリ |
| `/home/sprite/suisei/run.sh` | サービスが起動するスクリプト。`env` を読み、ログを `suisei.log` にも追記する |
| `/home/sprite/suisei/env` | 環境変数（権限 600）。`SUISEI_USER`、`SUISEI_PASSWORD`、`SUISEI_LISTEN=0.0.0.0:8080`、`SUISEI_DATA_DIR=/home/sprite/suisei/data`、`RUST_LOG=info`、`NO_COLOR=1` |
| `/home/sprite/suisei/suisei.log` | 再起動で消えないログ |

認証情報はサービス定義に載せず、`env` に置く。サービス定義は組織のトークンがあれば API で読めるため。

`run.sh` の中身:
```bash
#!/bin/bash
set -a
. /home/sprite/suisei/env
set +a
exec > >(tee -a /home/sprite/suisei/suisei.log) 2>&1
exec /home/sprite/suisei/suisei
```

## デプロイ
1. `server/` で `cargo build --release --target x86_64-unknown-linux-musl` を実行する。musl のターゲットと `musl-tools` が要る。
2. `PUT /fs/write?path=/home/sprite/suisei/suisei.new&workingDir=/home/sprite&mode=0755` でバイナリを送る。
3. `POST /exec` で sha256 を照合し、`suisei.new` を `suisei` に置き換える。
4. `POST /services/suisei/restart` で再起動する。

`fs/write` には `Content-Type: application/octet-stream` を付ける。curl の `--data-binary` はフォームの型を付け、400 や 413 で失敗する。

## サービスの登録
初回だけ `PUT /services/suisei` に次を送る。`http_port` で公開 URL に結び付く。
```json
{"cmd": "/home/sprite/suisei/run.sh", "args": [], "needs": [], "http_port": 8080, "dir": "/home/sprite/suisei"}
```
URL は Sprite の作成時に `url_settings.auth` を `public` にして公開した。Subsonic の認証で守る。

## ログ
- 直近のログは `GET /services/suisei/logs?lines=100`。
- 全体は `POST /exec` で `/home/sprite/suisei/suisei.log` を読む。
