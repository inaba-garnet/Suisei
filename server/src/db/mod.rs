//! DB 層。クエリはこのモジュールの中に閉じ込める（docs/server.md）。

pub mod annotation;
pub mod backup;
pub mod browse;
pub mod history;
mod id;
pub mod library;
pub mod playlist;
pub mod search;
pub mod session;

use std::path::Path;
use std::time::SystemTime;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};

pub use id::{IdKind, new_id};

/// PostgreSQL に移るときは、この型と接続の設定を差し替える。
pub type Pool = sqlx::SqlitePool;
pub type Connection = sqlx::SqliteConnection;

const DB_FILE: &str = "suisei.db";

pub static MIGRATOR: Migrator = sqlx::migrate!();

/// データの置き場所にある DB を開き、マイグレーションを適用する。
/// 未適用のマイグレーションがあれば、適用の前に `backup_dir` へ写す。
pub async fn open(data_dir: &Path, backup_dir: &Path) -> Result<Pool, sqlx::Error> {
    std::fs::create_dir_all(data_dir)?;
    let options = SqliteConnectOptions::new()
        .filename(data_dir.join(DB_FILE))
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = Pool::connect_with(options).await?;
    if let Some(path) =
        backup::before_migrate(&pool, &MIGRATOR, backup_dir, SystemTime::now()).await?
    {
        tracing::info!(path = %path.display(), "backed up database before migration");
    }
    migrate(&pool).await?;
    Ok(pool)
}

/// テスト用のメモリ上の DB。
pub async fn open_in_memory() -> Result<Pool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .in_memory(true)
        .foreign_keys(true);
    // メモリ上の DB は接続ごとに別物になるので、接続を一本に絞る
    let pool = sqlx::pool::PoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &Pool) -> Result<(), sqlx::Error> {
    MIGRATOR.run(pool).await?;
    Ok(())
}
