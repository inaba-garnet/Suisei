//! Web から変える設定の一行（docs/server.md の「設定」）。

use std::time::Duration;

use super::Pool;
use crate::settings::Settings;

pub async fn load(pool: &Pool) -> Result<Settings, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT scan_interval_secs, backup_interval_secs, backup_keep,
                  split_characters AS "split_characters: bool", spotify_client_id
           FROM settings WHERE id = 1"#
    )
    .fetch_one(pool)
    .await?;
    Ok(Settings {
        scan_interval: secs(row.scan_interval_secs),
        backup_interval: secs(row.backup_interval_secs),
        backup_keep: u32::try_from(row.backup_keep).unwrap_or(0),
        split_characters: row.split_characters,
        spotify_client_id: row.spotify_client_id,
    })
}

pub async fn save(pool: &Pool, settings: &Settings) -> Result<(), sqlx::Error> {
    let scan = i64::try_from(settings.scan_interval.as_secs()).unwrap_or(i64::MAX);
    let backup = i64::try_from(settings.backup_interval.as_secs()).unwrap_or(i64::MAX);
    sqlx::query!(
        "UPDATE settings SET scan_interval_secs = ?, backup_interval_secs = ?, backup_keep = ?,
             split_characters = ?, spotify_client_id = ?
         WHERE id = 1",
        scan,
        backup,
        settings.backup_keep,
        settings.split_characters,
        settings.spotify_client_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn secs(value: i64) -> Duration {
    Duration::from_secs(u64::try_from(value).unwrap_or(0))
}
