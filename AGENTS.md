# Suisei

Subsonic 互換の音楽サーバー（`server/`、Rust）と Web クライアント（`web/`、Nuxt）。

## 言語
- コードコメント、コミットメッセージ、PR の説明は日本語で書く。識別子は英語。

## コミット
- 形式は `<type>(<scope>): <要約>`。scope は `server` / `web` / `common`。
- 基本は scope ごとにコミットを分ける。分けられない変更やリポジトリ全体の設定は `common`。
- 本文には「なぜ」と「なにを」を端的に書く。コードを読めば分かることは書かない。
- 要約と本文を合わせて 140 字以内を目安にする。
- 例:
  ```
  feat(server): /search の POST メソッドを受け付ける

  一部クライアントは GET のクエリではなく POST で /search を叩くので、POST を受け付けるようにした
  ```

## Subsonic API
- 仕様の基準は OpenSubsonic（https://opensubsonic.netlify.app/）。
- エラーも HTTP 200 で返し、本文の `status="failed"` とエラーコードで伝える。
- 認証は平文パスワード（`p`）とトークン認証（`t` + `s`、MD5）の両方に対応する。
- `f=json` / `f=xml` の両方を返す。パスは `.view` の有無どちらでも受ける。
- GET のクエリだけでなく、POST のフォーム送信でもパラメータを受ける。
