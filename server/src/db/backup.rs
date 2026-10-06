//! DB のバックアップ（docs/server.md の「DB のバックアップ」）。

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sqlx::migrate::{Migrate, Migrator};

use super::Pool;

const PERIODIC_PREFIX: &str = "suisei-";
const PRE_MIGRATE_PREFIX: &str = "suisei-pre-migrate-";
const EXTENSION: &str = ".db";
const PRE_MIGRATE_KEEP: usize = 3;
/// 期限が過ぎたかを確かめる間隔。休止の後に早く気付けるよう、間隔そのものより短くする。
const CHECK_INTERVAL: Duration = Duration::from_secs(60);

/// 定期の写しの設定。
#[derive(Debug, Clone)]
pub struct Periodic {
    pub dir: PathBuf,
    /// `0` なら写さない。
    pub interval: Duration,
    /// `0` なら写さない。
    pub keep: usize,
}

impl Periodic {
    fn enabled(&self) -> bool {
        !self.interval.is_zero() && self.keep > 0
    }

    /// 期限が来るたびに写す。失敗しても次の確認でやり直す。
    pub async fn run(self, pool: Pool) {
        if !self.enabled() {
            tracing::info!("periodic database backup is disabled");
            return;
        }
        loop {
            match self.run_once(&pool, SystemTime::now()).await {
                Ok(Some(path)) => tracing::info!(path = %path.display(), "backed up database"),
                Ok(None) => {}
                Err(err) => tracing::error!(%err, "failed to back up database"),
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    }

    /// 最新の写しから間隔が過ぎていれば写し、古い世代を消す。写したらその場所を返す。
    pub async fn run_once(
        &self,
        pool: &Pool,
        now: SystemTime,
    ) -> Result<Option<PathBuf>, sqlx::Error> {
        // 止めたときは、残っている写しも消さない
        if !self.enabled() {
            return Ok(None);
        }
        // 時計が戻ったときに写さなくならないよう、未来の時刻の写しは数えない
        let latest = list(&self.dir, parse_periodic)?
            .into_iter()
            .map(|(at, _)| at)
            .filter(|at| *at <= now)
            .max();
        if latest.is_some_and(|at| at + self.interval > now) {
            return Ok(None);
        }
        let name = format!("{PERIODIC_PREFIX}{}{EXTENSION}", timestamp(now));
        let path = write(pool, &self.dir, &name).await?;
        prune(&self.dir, self.keep, parse_periodic)?;
        Ok(Some(path))
    }
}

/// 未適用のマイグレーションがあれば、適用の前に写す。写したらその場所を返す。
pub async fn before_migrate(
    pool: &Pool,
    migrator: &Migrator,
    dir: &Path,
    now: SystemTime,
) -> Result<Option<PathBuf>, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    let has_table: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)",
    )
    .bind(migrator.table_name.as_ref())
    .fetch_one(&mut *conn)
    .await?;
    // マイグレーションの表がなければ新しい DB で、守るデータがない
    if !has_table {
        return Ok(None);
    }
    let applied = conn.list_applied_migrations(&migrator.table_name).await?;
    drop(conn);
    let pending = migrator
        .iter()
        .filter(|m| m.migration_type.is_up_migration())
        .any(|m| !applied.iter().any(|a| a.version == m.version));
    if !pending {
        return Ok(None);
    }
    let version = applied.iter().map(|a| a.version).max().unwrap_or(0);
    let name = format!(
        "{PRE_MIGRATE_PREFIX}{version:04}-{}{EXTENSION}",
        timestamp(now)
    );
    let path = write(pool, dir, &name).await?;
    prune(dir, PRE_MIGRATE_KEEP, parse_pre_migrate)?;
    Ok(Some(path))
}

async fn write(pool: &Pool, dir: &Path, name: &str) -> Result<PathBuf, sqlx::Error> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let tmp = dir.join(format!("{name}.tmp"));
    // VACUUM INTO は書き出し先があると失敗するので、前に止まった残りを消す
    match std::fs::remove_file(&tmp) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    }
    let tmp_str = tmp.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("backup path is not UTF-8: {}", tmp.display()),
        )
    })?;
    // スキーマに依らない文なので、マクロで照合せずに書く
    if let Err(err) = sqlx::query("VACUUM INTO ?")
        .bind(tmp_str)
        .execute(pool)
        .await
    {
        let _ = std::fs::remove_file(&tmp);
        return Err(err);
    }
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

/// 名前から時刻を読めた写しを、新しい順に `keep` 個だけ残す。
fn prune(dir: &Path, keep: usize, parse: fn(&str) -> Option<SystemTime>) -> std::io::Result<()> {
    let mut backups = list(dir, parse)?;
    backups.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    for (_, path) in backups.into_iter().skip(keep) {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn list(
    dir: &Path,
    parse: fn(&str) -> Option<SystemTime>,
) -> std::io::Result<Vec<(SystemTime, PathBuf)>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Some(at) = entry.file_name().to_str().and_then(parse) else {
            continue;
        };
        backups.push((at, entry.path()));
    }
    Ok(backups)
}

fn parse_periodic(name: &str) -> Option<SystemTime> {
    let stamp = name
        .strip_prefix(PERIODIC_PREFIX)?
        .strip_suffix(EXTENSION)?;
    parse_timestamp(stamp)
}

fn parse_pre_migrate(name: &str) -> Option<SystemTime> {
    let rest = name
        .strip_prefix(PRE_MIGRATE_PREFIX)?
        .strip_suffix(EXTENSION)?;
    let (version, stamp) = rest.split_once('-')?;
    version.parse::<i64>().ok()?;
    parse_timestamp(stamp)
}

/// `20261005T120000Z` の形。名前の順が時刻の順になり、Windows や SMB でも使えない `:` を含まない。
fn timestamp(at: SystemTime) -> String {
    humantime::format_rfc3339_seconds(at)
        .to_string()
        .replace(['-', ':'], "")
}

fn parse_timestamp(stamp: &str) -> Option<SystemTime> {
    let b = stamp.as_bytes();
    if b.len() != 16 || b[8] != b'T' || b[15] != b'Z' {
        return None;
    }
    let s = |r: std::ops::Range<usize>| stamp.get(r);
    let rfc3339 = format!(
        "{}-{}-{}T{}:{}:{}Z",
        s(0..4)?,
        s(4..6)?,
        s(6..8)?,
        s(9..11)?,
        s(11..13)?,
        s(13..15)?
    );
    humantime::parse_rfc3339(&rfc3339).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_round_trips() {
        let at = humantime::parse_rfc3339("2026-10-05T12:34:56Z").unwrap();
        assert_eq!(timestamp(at), "20261005T123456Z");
        assert_eq!(parse_timestamp("20261005T123456Z"), Some(at));
    }

    #[test]
    fn names_are_told_apart() {
        let at = humantime::parse_rfc3339("2026-10-05T00:00:00Z").unwrap();
        assert_eq!(parse_periodic("suisei-20261005T000000Z.db"), Some(at));
        assert_eq!(
            parse_periodic("suisei-pre-migrate-0011-20261005T000000Z.db"),
            None
        );
        assert_eq!(parse_periodic("suisei-20261005T000000Z.db.tmp"), None);
        assert_eq!(
            parse_pre_migrate("suisei-pre-migrate-0011-20261005T000000Z.db"),
            Some(at)
        );
        assert_eq!(parse_pre_migrate("suisei-20261005T000000Z.db"), None);
    }
}
