//! Spotify の Web API とトークンの呼び出し。

use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::RETRY_AFTER;
use serde::Deserialize;

use crate::db::spotify::SpotifyTrack;

/// Spotify の認可の戻り先。利用者が Spotify のアプリに同じ値を登録する（docs/spotify.md の「認可の戻り先」）。
pub const REDIRECT_URI: &str = "http://127.0.0.1:27533/callback";
pub const SCOPE: &str = "user-library-read";
/// `GET /v1/me/tracks` の一度に読める上限
const PAGE_SIZE: usize = 50;
/// これより長く待てと言われたら諦める。次の取り込みでやり直す
const MAX_RETRY_AFTER: Duration = Duration::from_secs(120);

/// 呼び出し先。テストでは偽物のサーバーに向ける。
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub accounts: String,
    pub api: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            accounts: "https://accounts.spotify.com".into(),
            api: "https://api.spotify.com".into(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// リフレッシュトークンが取り消されたか切れた
    InvalidGrant,
    /// アクセストークンが切れた
    Unauthorized,
    Status(StatusCode, String),
    Http(reqwest::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGrant => write!(f, "authorization was revoked or expired"),
            Self::Unauthorized => write!(f, "access token was rejected"),
            Self::Status(status, body) => write!(f, "spotify returned {status}: {body}"),
            Self::Http(err) => write!(f, "{err}"),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub expires_in: u64,
    /// リフレッシュで返らなければ、前のものを使い続ける
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoints: Endpoints,
}

impl Client {
    pub fn new(endpoints: Endpoints) -> Self {
        // 暗号の実装は ring だけを入れているので、明示して使う。二度目以降の呼び出しは失敗するが害はない
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .user_agent(concat!("Suisei/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("HTTP クライアントを作れる");
        Self { http, endpoints }
    }

    pub fn authorize_url(&self, client_id: &str, state: &str, challenge: &str) -> String {
        reqwest::Url::parse_with_params(
            &format!("{}/authorize", self.endpoints.accounts),
            [
                ("client_id", client_id),
                ("response_type", "code"),
                ("redirect_uri", REDIRECT_URI),
                ("code_challenge_method", "S256"),
                ("code_challenge", challenge),
                ("state", state),
                ("scope", SCOPE),
            ],
        )
        .expect("認可の URL は正しい")
        .into()
    }

    /// 認可のコードをトークンに換える。
    pub async fn exchange_code(
        &self,
        client_id: &str,
        code: &str,
        verifier: &str,
    ) -> Result<Token, Error> {
        self.token(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ])
        .await
    }

    pub async fn refresh(&self, client_id: &str, refresh_token: &str) -> Result<Token, Error> {
        self.token(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ])
        .await
    }

    async fn token(&self, form: &[(&str, &str)]) -> Result<Token, Error> {
        let response = self
            .http
            .post(format!("{}/api/token", self.endpoints.accounts))
            .form(form)
            .send()
            .await?;
        let status = response.status();
        if status.is_success() {
            return Ok(response.json().await?);
        }
        let body = response.text().await.unwrap_or_default();
        #[derive(Deserialize)]
        struct TokenError {
            error: String,
        }
        if serde_json::from_str::<TokenError>(&body).is_ok_and(|e| e.error == "invalid_grant") {
            return Err(Error::InvalidGrant);
        }
        Err(Error::Status(status, body))
    }

    /// お気に入りの曲の一ページ。次のページがあれば true も返す。
    pub async fn saved_tracks(
        &self,
        access_token: &str,
        offset: usize,
    ) -> Result<(Vec<SpotifyTrack>, bool), Error> {
        let url = format!("{}/v1/me/tracks", self.endpoints.api);
        loop {
            let response = self
                .http
                .get(&url)
                .bearer_auth(access_token)
                .query(&[("limit", PAGE_SIZE), ("offset", offset)])
                .send()
                .await?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                let wait = retry_after(response.headers());
                if wait > MAX_RETRY_AFTER {
                    return Err(Error::Status(status, format!("retry after {wait:?}")));
                }
                tracing::info!(?wait, "spotify rate limited; waiting");
                tokio::time::sleep(wait).await;
                continue;
            }
            if status == StatusCode::UNAUTHORIZED {
                return Err(Error::Unauthorized);
            }
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(Error::Status(status, body));
            }
            let page: Page = response.json().await?;
            let tracks = page
                .items
                .into_iter()
                .filter_map(Item::into_track)
                .collect();
            return Ok((tracks, page.next.is_some()));
        }
    }
}

/// 秒数だけを受ける。Spotify は日時の形で返さない。
fn retry_after(headers: &reqwest::header::HeaderMap) -> Duration {
    let secs = headers
        .get(RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(1);
    Duration::from_secs(secs.max(1))
}

#[derive(Deserialize)]
struct Page {
    items: Vec<Item>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct Item {
    added_at: String,
    track: Option<Track>,
}

#[derive(Deserialize)]
struct Track {
    /// Spotify に取り込んだローカルファイルの曲は ID を持たない
    id: Option<String>,
    name: String,
    duration_ms: i64,
    disc_number: Option<i64>,
    track_number: Option<i64>,
    #[serde(default)]
    external_ids: ExternalIds,
    #[serde(default)]
    artists: Vec<Named>,
    album: Named,
}

#[derive(Deserialize, Default)]
struct ExternalIds {
    isrc: Option<String>,
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

impl Item {
    fn into_track(self) -> Option<SpotifyTrack> {
        let track = self.track?;
        let added_at = humantime::parse_rfc3339_weak(&self.added_at)
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some(SpotifyTrack {
            spotify_id: track.id?,
            isrc: track
                .external_ids
                .isrc
                .filter(|isrc| !isrc.trim().is_empty()),
            title: track.name,
            artists: track.artists.into_iter().map(|a| a.name).collect(),
            album: track.album.name,
            disc_number: track.disc_number,
            track_number: track.track_number,
            duration_ms: track.duration_ms,
            added_at: i64::try_from(added_at.as_millis()).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_is_parsed() {
        let page: Page = serde_json::from_str(
            r#"{"items":[
                {"added_at":"2024-05-01T12:00:00Z","track":{"id":"abc","name":"曲",
                 "duration_ms":200000,"disc_number":1,"track_number":2,
                 "external_ids":{"isrc":"JPXX01500001"},
                 "artists":[{"name":"歌手A"},{"name":"歌手B"}],"album":{"name":"盤"}}},
                {"added_at":"2024-05-01T12:00:00Z","track":{"id":null,"name":"ローカル",
                 "duration_ms":1,"disc_number":0,"track_number":0,"artists":[],"album":{"name":""}}}
               ],"next":null}"#,
        )
        .unwrap();
        assert!(page.next.is_none());
        let tracks: Vec<_> = page
            .items
            .into_iter()
            .filter_map(Item::into_track)
            .collect();
        assert_eq!(
            tracks,
            [SpotifyTrack {
                spotify_id: "abc".into(),
                isrc: Some("JPXX01500001".into()),
                title: "曲".into(),
                artists: vec!["歌手A".into(), "歌手B".into()],
                album: "盤".into(),
                disc_number: Some(1),
                track_number: Some(2),
                duration_ms: 200_000,
                added_at: 1_714_564_800_000,
            }]
        );
    }
}
