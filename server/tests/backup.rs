//! DB のバックアップを一時ディレクトリに書き、写しから読めるかを確かめる。

use std::path::Path;
use std::time::{Duration, SystemTime};

use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteConnectOptions;
use suisei::db::backup::{self, Periodic};
use suisei::db::{self, MIGRATOR, Pool};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

fn at(rfc3339: &str) -> SystemTime {
    humantime::parse_rfc3339(rfc3339).unwrap()
}

async fn connect(path: &Path) -> Pool {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    Pool::connect_with(options).await.unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

async fn applied_versions(pool: &Pool) -> Vec<i64> {
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn periodic_backup_waits_for_interval_and_keeps_generations() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = dir.path().join("backup");
    let pool = db::open(dir.path(), &backup_dir).await.unwrap();
    sqlx::query("INSERT INTO playlist (id, name, comment, public, created_at, changed_at) VALUES ('pl-00000000', 'お気に入り', '', 0, 0, 0)")
        .execute(&pool)
        .await
        .unwrap();
    let periodic = Periodic {
        dir: backup_dir.clone(),
        interval: DAY,
        keep: 2,
    };

    let first = periodic
        .run_once(&pool, at("2026-10-05T00:00:00Z"))
        .await
        .unwrap()
        .unwrap();
    assert!(
        periodic
            .run_once(&pool, at("2026-10-05T23:59:59Z"))
            .await
            .unwrap()
            .is_none(),
        "間隔が過ぎる前に写した"
    );
    periodic
        .run_once(&pool, at("2026-10-06T00:00:00Z"))
        .await
        .unwrap()
        .unwrap();
    periodic
        .run_once(&pool, at("2026-10-08T12:00:00Z"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        names(&backup_dir),
        ["suisei-20261006T000000Z.db", "suisei-20261008T120000Z.db"]
    );
    assert!(!first.exists(), "古い世代が残った");

    // 写しを DB として開けて、書いた内容が入っている
    let restored = connect(&backup_dir.join("suisei-20261008T120000Z.db")).await;
    let name: String = sqlx::query_scalar("SELECT name FROM playlist")
        .fetch_one(&restored)
        .await
        .unwrap();
    assert_eq!(name, "お気に入り");
}

#[tokio::test]
async fn periodic_backup_ignores_backups_from_the_future() {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(dir.path(), &dir.path().join("backup"))
        .await
        .unwrap();
    let periodic = Periodic {
        dir: dir.path().join("backup"),
        interval: DAY,
        keep: 7,
    };
    periodic
        .run_once(&pool, at("2027-01-01T00:00:00Z"))
        .await
        .unwrap()
        .unwrap();
    // 時計が戻っても写し続ける
    assert!(
        periodic
            .run_once(&pool, at("2026-10-05T00:00:00Z"))
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn zero_interval_or_keep_disables_periodic_backup() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = dir.path().join("backup");
    let pool = db::open(dir.path(), &backup_dir).await.unwrap();
    let enabled = Periodic {
        dir: backup_dir.clone(),
        interval: DAY,
        keep: 7,
    };
    enabled
        .run_once(&pool, at("2026-10-01T00:00:00Z"))
        .await
        .unwrap()
        .unwrap();
    for (interval, keep) in [(Duration::ZERO, 7), (DAY, 0)] {
        let disabled = Periodic {
            dir: backup_dir.clone(),
            interval,
            keep,
        };
        assert!(
            disabled
                .run_once(&pool, at("2026-10-05T00:00:00Z"))
                .await
                .unwrap()
                .is_none(),
            "interval={interval:?} keep={keep} で写した"
        );
    }
    // 止めても、残っている写しは消さない
    assert_eq!(names(&backup_dir), ["suisei-20261001T000000Z.db"]);
}

#[tokio::test]
async fn new_database_is_not_backed_up_before_migration() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = dir.path().join("backup");
    db::open(dir.path(), &backup_dir).await.unwrap();
    db::open(dir.path(), &backup_dir).await.unwrap();
    assert!(
        !backup_dir.exists(),
        "適用するマイグレーションがないのに写した"
    );
}

#[tokio::test]
async fn pending_migration_is_backed_up_first() {
    let dir = tempfile::tempdir().unwrap();
    let backup_dir = dir.path().join("backup");
    let pool = connect(&dir.path().join("suisei.db")).await;
    let latest = MIGRATOR.iter().map(|m| m.version).max().unwrap();
    let old = Migrator::with_migrations(
        MIGRATOR
            .iter()
            .filter(|m| m.version < latest)
            .cloned()
            .collect(),
    );
    old.run(&pool).await.unwrap();

    for (i, now) in ["2026-10-01", "2026-10-02", "2026-10-03", "2026-10-04"]
        .iter()
        .enumerate()
    {
        let path = backup::before_migrate(
            &pool,
            &MIGRATOR,
            &backup_dir,
            at(&format!("{now}T00:00:00Z")),
        )
        .await
        .unwrap()
        .unwrap();
        if i == 3 {
            // 写しは適用前の版のまま
            let restored = connect(&path).await;
            assert_eq!(
                applied_versions(&restored).await.last(),
                Some(&(latest - 1))
            );
        }
    }
    let previous = latest - 1;
    assert_eq!(
        names(&backup_dir),
        [
            format!("suisei-pre-migrate-{previous:04}-20261002T000000Z.db"),
            format!("suisei-pre-migrate-{previous:04}-20261003T000000Z.db"),
            format!("suisei-pre-migrate-{previous:04}-20261004T000000Z.db"),
        ]
    );

    drop(pool);
    db::open(dir.path(), &backup_dir).await.unwrap();
    let pool = connect(&dir.path().join("suisei.db")).await;
    assert_eq!(applied_versions(&pool).await.last(), Some(&latest));
    assert!(
        backup::before_migrate(&pool, &MIGRATOR, &backup_dir, SystemTime::now())
            .await
            .unwrap()
            .is_none()
    );
}
