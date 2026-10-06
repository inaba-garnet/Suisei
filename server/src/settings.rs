//! Web から変える設定（docs/server.md の「設定」）。DB に持ち、変わったら動いている処理に知らせる。

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use crate::db::{self, Pool};
use crate::tags;

/// 定期スキャンの間隔の下限。短すぎると NAS の上でスキャンが休まず走るため。
pub const MIN_SCAN_INTERVAL: Duration = Duration::from_secs(60);
/// 定期の写しの間隔の下限。写すたびに DB の大きさだけ書き込むため。
pub const MIN_BACKUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
pub const MAX_BACKUP_KEEP: u32 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 定期スキャンの間隔。前のスキャンが終わってから数える。`0` で起動時の一度だけ
    pub scan_interval: Duration,
    /// 定期の写しの間隔。`0` で止める
    pub backup_interval: Duration,
    /// 残す定期の写しの数。`0` で止める
    pub backup_keep: u32,
    /// `キャラクター(CV:声優)` の形の名前を分ける
    pub split_characters: bool,
    /// Spotify のアプリの Client ID。なければ Spotify 連携を使わない（docs/spotify.md）
    pub spotify_client_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scan_interval: Duration::from_secs(60 * 60),
            backup_interval: Duration::from_secs(24 * 60 * 60),
            backup_keep: 7,
            split_characters: true,
            spotify_client_id: None,
        }
    }
}

impl Settings {
    pub fn tag_options(&self) -> tags::Options {
        tags::Options {
            split_characters: self.split_characters,
        }
    }

    /// 受け付けない値なら、どの項目かを返す。
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.scan_interval.is_zero() && self.scan_interval < MIN_SCAN_INTERVAL {
            return Err("scanInterval");
        }
        if !self.backup_interval.is_zero() && self.backup_interval < MIN_BACKUP_INTERVAL {
            return Err("backupInterval");
        }
        if self.backup_keep > MAX_BACKUP_KEEP {
            return Err("backupKeep");
        }
        // Spotify の Client ID は 32 文字の 16 進数。貼り間違いを早く知らせるため、英数字に限る
        if let Some(id) = &self.spotify_client_id
            && (id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Err("spotifyClientId");
        }
        Ok(())
    }
}

/// 設定の置き場所。読むときはメモリの値を使い、変えたら DB に書いて受け口に知らせる。
#[derive(Debug)]
pub struct Store {
    pool: Pool,
    tx: watch::Sender<Settings>,
}

impl Store {
    /// DB を読まずに `settings` で作る。テスト用。
    pub fn new(pool: Pool, settings: Settings) -> Arc<Self> {
        Arc::new(Self {
            pool,
            tx: watch::Sender::new(settings),
        })
    }

    /// DB の設定を読んで作る。
    pub async fn load(pool: Pool) -> Result<Arc<Self>, sqlx::Error> {
        let settings = db::settings::load(&pool).await?;
        Ok(Self::new(pool, settings))
    }

    pub fn get(&self) -> Settings {
        self.tx.borrow().clone()
    }

    /// 変わるたびに知らせる受け口。
    pub fn subscribe(&self) -> watch::Receiver<Settings> {
        self.tx.subscribe()
    }

    /// 設定を置き換える。受け付けない値なら、どの項目かを返す。
    pub async fn update(&self, settings: Settings) -> Result<(), UpdateError> {
        settings.validate().map_err(UpdateError::Invalid)?;
        db::settings::save(&self.pool, &settings)
            .await
            .map_err(UpdateError::Db)?;
        self.tx.send_if_modified(|current| {
            let changed = *current != settings;
            *current = settings;
            changed
        });
        Ok(())
    }
}

#[derive(Debug)]
pub enum UpdateError {
    Invalid(&'static str),
    Db(sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ranges() {
        assert_eq!(Settings::default().validate(), Ok(()));
        let short = Settings {
            scan_interval: Duration::from_secs(10),
            ..Settings::default()
        };
        assert_eq!(short.validate(), Err("scanInterval"));
        let off = Settings {
            scan_interval: Duration::ZERO,
            backup_interval: Duration::ZERO,
            backup_keep: 0,
            ..Settings::default()
        };
        assert_eq!(off.validate(), Ok(()));
        let id = |id: &str| Settings {
            spotify_client_id: Some(id.into()),
            ..Settings::default()
        };
        assert_eq!(id("0123456789abcdef0123456789abcdef").validate(), Ok(()));
        assert_eq!(id("https://example.com").validate(), Err("spotifyClientId"));
        assert_eq!(id("").validate(), Err("spotifyClientId"));
    }
}
