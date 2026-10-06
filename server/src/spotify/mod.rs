//! Spotify 連携。お気に入りの曲を読み、ローカルの同じ曲をお気に入りにする（docs/spotify.md）。

mod client;
mod matching;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex as AsyncMutex, watch};

use crate::db::{self, Pool};
use crate::settings;
use client::Client;
pub use client::{Endpoints, REDIRECT_URI};

/// 認可を始めてから、URL を貼り付けるまでの期限。
const PENDING_LIFETIME: Duration = Duration::from_secs(10 * 60);
/// 定期の取り込みで Spotify を呼ぶ最短の間隔。
const FETCH_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// アクセストークンの期限より、これだけ早く取り直す。
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum Error {
    NotConnected,
    /// 接続が切れた。接続し直してもらう
    Revoked,
    Spotify(client::Error),
    Db(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => write!(f, "not connected to spotify"),
            Self::Revoked => write!(f, "spotify authorization was revoked; connect again"),
            Self::Spotify(err) => write!(f, "{err}"),
            Self::Db(err) => write!(f, "{err}"),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::Db(err)
    }
}

/// 貼り付けた URL を受け付けなかった理由。
#[derive(Debug)]
pub enum ConnectError {
    /// URL として読めないか、`code` と `state` がない
    InvalidUrl,
    /// Spotify で許可しなかった（`error=access_denied` など）
    Denied(String),
    /// 始めた認可と `state` が合わないか、期限が切れた
    UnknownState,
    Failed(Error),
}

/// 最後の取り込みの結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSync {
    /// 終わった日時（UNIX 時刻のミリ秒）
    pub at: i64,
    /// 失敗したときの理由
    pub error: Option<String>,
    /// Spotify から読んだ曲の数。Spotify を呼ばずに付け直しただけなら None
    pub fetched: Option<usize>,
    /// 新しく対応を付けた曲の数
    pub matched: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub connected: bool,
    pub syncing: bool,
    pub last_sync: Option<LastSync>,
    pub counts: db::spotify::Counts,
}

#[derive(Debug)]
struct Pending {
    verifier: String,
    client_id: String,
    at: Instant,
}

#[derive(Debug)]
pub struct Spotify {
    pool: Pool,
    client: Client,
    /// Client ID とタグの解釈は設定の画面で変わるので、使うたびに読む
    settings: Arc<settings::Store>,
    /// 始めた認可。`state` から PKCE の verifier、使った Client ID、始めた時刻を引く
    pending: Mutex<HashMap<String, Pending>>,
    access_token: AsyncMutex<Option<(String, Instant)>>,
    /// 取り込みは一度に一つだけ
    lock: Arc<AsyncMutex<()>>,
    syncing: AtomicBool,
    last: Mutex<Option<LastSync>>,
}

impl Spotify {
    pub fn new(pool: Pool, settings: Arc<settings::Store>, endpoints: Endpoints) -> Arc<Self> {
        Arc::new(Self {
            pool,
            client: Client::new(endpoints),
            settings,
            pending: Mutex::default(),
            access_token: AsyncMutex::default(),
            lock: Arc::default(),
            syncing: AtomicBool::new(false),
            last: Mutex::default(),
        })
    }

    /// 設定の Client ID。なければ Spotify 連携を使わない。
    pub fn client_id(&self) -> Option<String> {
        self.settings.get().spotify_client_id
    }

    /// 認可を始め、Spotify の認可の画面の URL を返す。Client ID がなければ None。
    pub fn authorize_url(&self) -> Option<String> {
        let client_id = self.client_id()?;
        let state = hex::encode(random_bytes::<16>());
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes::<48>());
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(&verifier));
        let mut pending = self.pending.lock().expect("認可のロックが壊れた");
        pending.retain(|_, p| p.at.elapsed() < PENDING_LIFETIME);
        let url = self.client.authorize_url(&client_id, &state, &challenge);
        pending.insert(
            state,
            Pending {
                verifier,
                client_id,
                at: Instant::now(),
            },
        );
        Some(url)
    }

    /// 認可の後にブラウザが開けなかったページの URL を受け、接続する。
    pub async fn connect(&self, pasted: &str) -> Result<(), ConnectError> {
        let url = reqwest::Url::parse(pasted.trim()).map_err(|_| ConnectError::InvalidUrl)?;
        let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
        if let Some(error) = query.get("error") {
            return Err(ConnectError::Denied(error.clone()));
        }
        let (Some(code), Some(state)) = (query.get("code"), query.get("state")) else {
            return Err(ConnectError::InvalidUrl);
        };
        let pending = {
            let mut pending = self.pending.lock().expect("認可のロックが壊れた");
            match pending.remove(state) {
                Some(p) if p.at.elapsed() < PENDING_LIFETIME => p,
                _ => return Err(ConnectError::UnknownState),
            }
        };
        // 認可を始めた後に Client ID を変えたなら、古いアプリで許可した URL なのでつながない
        if self.client_id().as_deref() != Some(pending.client_id.as_str()) {
            return Err(ConnectError::UnknownState);
        }
        let token = self
            .client
            .exchange_code(&pending.client_id, code, &pending.verifier)
            .await
            .map_err(|err| ConnectError::Failed(Error::Spotify(err)))?;
        let Some(refresh_token) = &token.refresh_token else {
            return Err(ConnectError::Failed(Error::Spotify(client::Error::Status(
                reqwest::StatusCode::OK,
                "no refresh token".into(),
            ))));
        };
        db::spotify::connect(&self.pool, refresh_token, now_ms())
            .await
            .map_err(|err| ConnectError::Failed(err.into()))?;
        *self.access_token.lock().await = Some(expiry(&token));
        tracing::info!("connected to spotify");
        Ok(())
    }

    /// 接続を切る。対応表とローカルのお気に入りは残す。
    pub async fn disconnect(&self) -> Result<(), sqlx::Error> {
        let connected = db::spotify::disconnect(&self.pool).await?;
        *self.access_token.lock().await = None;
        if connected {
            tracing::info!("disconnected from spotify");
        }
        Ok(())
    }

    pub async fn status(&self) -> Result<Status, sqlx::Error> {
        // ロックを持ったまま待たないよう、先に写す
        let last_sync = self.last.lock().expect("状態のロックが壊れた").clone();
        Ok(Status {
            connected: db::spotify::account(&self.pool).await?.is_some(),
            syncing: self.syncing.load(Ordering::Acquire),
            last_sync,
            counts: db::spotify::counts(&self.pool).await?,
        })
    }

    /// 取り込みが走っていなければ、バックグラウンドで始める。始めたら true。
    pub fn start_sync(self: &Arc<Self>) -> bool {
        let Some(guard) = self.begin() else {
            return false;
        };
        let this = Arc::clone(self);
        tokio::spawn(async move {
            let result = this.fetch_and_match().await;
            this.finish(result);
            drop(guard);
        });
        true
    }

    /// スキャンが終わるたびに、対応を付け直す。定期スキャンが有効で、前に読んでから 60 分たっていれば Spotify から読み直す。
    pub async fn follow_scans(self: Arc<Self>, mut finished: watch::Receiver<u64>) {
        while finished.changed().await.is_ok() {
            let Some(guard) = self.begin() else {
                continue;
            };
            let result = match self.should_fetch().await {
                Ok(true) => self.fetch_and_match().await,
                Ok(false) => self.rematch().await.map(|matched| (None, matched)),
                Err(err) => Err(err.into()),
            };
            self.finish(result);
            drop(guard);
        }
    }

    async fn should_fetch(&self) -> Result<bool, sqlx::Error> {
        if self.settings.get().scan_interval.is_zero() || self.client_id().is_none() {
            return Ok(false);
        }
        let Some((_, fetched_at)) = db::spotify::account(&self.pool).await? else {
            return Ok(false);
        };
        Ok(fetched_at.is_none_or(|at| now_ms() - at >= millis(FETCH_INTERVAL)))
    }

    fn begin(&self) -> Option<tokio::sync::OwnedMutexGuard<()>> {
        let guard = Arc::clone(&self.lock).try_lock_owned().ok()?;
        self.syncing.store(true, Ordering::Release);
        Some(guard)
    }

    fn finish(&self, result: Result<(Option<usize>, usize), Error>) {
        let last = match result {
            Ok((fetched, matched)) => {
                tracing::info!(?fetched, matched, "spotify sync finished");
                LastSync {
                    at: now_ms(),
                    error: None,
                    fetched,
                    matched,
                }
            }
            Err(err) => {
                tracing::warn!(%err, "spotify sync failed");
                LastSync {
                    at: now_ms(),
                    error: Some(err.to_string()),
                    fetched: None,
                    matched: 0,
                }
            }
        };
        *self.last.lock().expect("状態のロックが壊れた") = Some(last);
        self.syncing.store(false, Ordering::Release);
    }

    /// Spotify からお気に入りを全件読み、対応表を置き換えて対応を付け直す。読んだ数と新しく対応した数を返す。
    async fn fetch_and_match(&self) -> Result<(Option<usize>, usize), Error> {
        db::spotify::set_fetched_at(&self.pool, now_ms()).await?;
        let mut tracks = Vec::new();
        loop {
            let (page, more) = self.saved_tracks(tracks.len()).await?;
            let empty = page.is_empty();
            tracks.extend(page);
            if !more || empty {
                break;
            }
        }
        // 同じ曲が二度返ることはないはずだが、主キーの重複で全体を失わないよう先のものを残す
        let mut seen = std::collections::HashSet::new();
        tracks.retain(|t| seen.insert(t.spotify_id.clone()));
        db::spotify::replace_tracks(&self.pool, &tracks).await?;
        let matched = self.rematch().await?;
        Ok((Some(tracks.len()), matched))
    }

    /// アクセストークンが切れていれば一度だけ取り直して、一ページ読む。
    async fn saved_tracks(
        &self,
        offset: usize,
    ) -> Result<(Vec<db::spotify::SpotifyTrack>, bool), Error> {
        let token = self.access_token(false).await?;
        match self.client.saved_tracks(&token, offset).await {
            Err(client::Error::Unauthorized) => {
                let token = self.access_token(true).await?;
                self.client
                    .saved_tracks(&token, offset)
                    .await
                    .map_err(Error::Spotify)
            }
            result => result.map_err(Error::Spotify),
        }
    }

    async fn access_token(&self, force: bool) -> Result<String, Error> {
        let mut cached = self.access_token.lock().await;
        if !force
            && let Some((token, expires)) = cached.as_ref()
            && Instant::now() < *expires
        {
            return Ok(token.clone());
        }
        let Some((refresh_token, _)) = db::spotify::account(&self.pool).await? else {
            return Err(Error::NotConnected);
        };
        let Some(client_id) = self.client_id() else {
            return Err(Error::NotConnected);
        };
        let token = match self.client.refresh(&client_id, &refresh_token).await {
            Ok(token) => token,
            Err(client::Error::InvalidGrant) => {
                // 接続が切れた。切れたままの接続で取り込みを繰り返さないよう、接続を消す
                db::spotify::disconnect(&self.pool).await?;
                *cached = None;
                return Err(Error::Revoked);
            }
            Err(err) => return Err(Error::Spotify(err)),
        };
        if let Some(rotated) = &token.refresh_token {
            db::spotify::update_refresh_token(&self.pool, rotated).await?;
        }
        let entry = expiry(&token);
        let access = entry.0.clone();
        *cached = Some(entry);
        Ok(access)
    }

    /// 対応のない行に対応を付け、ローカルの曲をお気に入りにする。新しく対応した数を返す。
    async fn rematch(&self) -> Result<usize, Error> {
        let unmatched = db::spotify::unmatched(&self.pool).await?;
        if unmatched.is_empty() {
            return Ok(0);
        }
        let index = matching::Index::new(db::spotify::local_tracks(&self.pool).await?);
        let options = self.settings.get().tag_options();
        let links: Vec<_> = unmatched
            .iter()
            .filter_map(|track| {
                let (track_id, method) = index.find(track, options)?;
                Some((track.spotify_id.as_str(), track_id, method))
            })
            .collect();
        if links.is_empty() {
            return Ok(0);
        }
        Ok(db::spotify::link_many(&self.pool, &links).await?)
    }
}

fn expiry(token: &client::Token) -> (String, Instant) {
    let lifetime = Duration::from_secs(token.expires_in).saturating_sub(EXPIRY_MARGIN);
    (token.access_token.clone(), Instant::now() + lifetime)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("failed to get random bytes");
    bytes
}

fn now_ms() -> i64 {
    millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

fn millis(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
