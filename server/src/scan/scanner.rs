//! スキャンを一度に一つだけ走らせ、`getScanStatus` に返す状態を持つ。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use super::{Mode, run_with_progress};
use crate::db::Pool;

#[derive(Debug)]
pub struct Scanner {
    pool: Pool,
    music_dir: PathBuf,
    /// スキャン中は取られている
    lock: Arc<AsyncMutex<()>>,
    scanning: AtomicBool,
    /// スキャン中に見つけた音声の数
    progress: Arc<AtomicUsize>,
    last: Mutex<Option<Finished>>,
}

#[derive(Debug, Clone, Copy)]
struct Finished {
    at: SystemTime,
    files: usize,
    folders: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub scanning: bool,
    /// スキャン中は見つけた音声の数、それ以外はライブラリのファイル数
    pub count: usize,
    pub folder_count: usize,
    /// 最後に成功したスキャンの終了時刻
    pub last_scan: Option<SystemTime>,
}

impl Scanner {
    pub fn new(pool: Pool, music_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            pool,
            music_dir,
            lock: Arc::default(),
            scanning: AtomicBool::new(false),
            progress: Arc::default(),
            last: Mutex::new(None),
        })
    }

    /// スキャン中でなければ、バックグラウンドで始める。始めたら true。
    pub fn start(self: &Arc<Self>, mode: Mode) -> bool {
        let Some(guard) = self.begin() else {
            return false;
        };
        let this = Arc::clone(self);
        tokio::spawn(async move { this.scan(mode, guard).await });
        true
    }

    /// 走っているスキャンが終わるのを待つ。
    pub async fn wait(&self) {
        drop(self.lock.lock().await);
    }

    /// すぐに一度スキャンし、`interval` ごとに繰り返す。間隔は前のスキャンが終わってから数える。
    /// `interval` が 0 なら一度だけ。手動のスキャンが走っていれば、その回は飛ばす。
    pub async fn run_periodically(self: Arc<Self>, interval: Duration) {
        loop {
            if let Some(guard) = self.begin() {
                self.scan(Mode::Quick, guard).await;
            }
            if interval.is_zero() {
                return;
            }
            tokio::time::sleep(interval).await;
        }
    }

    pub fn status(&self) -> Status {
        let scanning = self.scanning.load(Ordering::Acquire);
        let last = *self.last.lock().expect("状態のロックが壊れた");
        Status {
            scanning,
            count: if scanning {
                self.progress.load(Ordering::Relaxed)
            } else {
                last.map_or(0, |l| l.files)
            },
            folder_count: last.map_or(0, |l| l.folders),
            last_scan: last.map(|l| l.at),
        }
    }

    /// ロックを取り、スキャン中の印を付ける。呼んだ直後の `status` にも反映させるため、タスクを起こす前に行う。
    fn begin(&self) -> Option<OwnedMutexGuard<()>> {
        let guard = Arc::clone(&self.lock).try_lock_owned().ok()?;
        self.progress.store(0, Ordering::Relaxed);
        self.scanning.store(true, Ordering::Release);
        Some(guard)
    }

    async fn scan(&self, mode: Mode, _guard: OwnedMutexGuard<()>) {
        let started = Instant::now();
        let result = run_with_progress(
            &self.pool,
            &self.music_dir,
            mode,
            Arc::clone(&self.progress),
        )
        .await;
        match result {
            Ok(summary) => {
                tracing::info!(
                    ?mode,
                    files = summary.files,
                    read = summary.read,
                    failed = summary.failed,
                    tracks = summary.tracks,
                    albums = summary.albums,
                    artists = summary.artists,
                    elapsed_ms = started.elapsed().as_millis(),
                    "scan finished"
                );
                *self.last.lock().expect("状態のロックが壊れた") = Some(Finished {
                    at: SystemTime::now(),
                    files: summary.files,
                    folders: summary.folders,
                });
            }
            Err(err) => tracing::error!(?mode, error = %err, "scan failed"),
        }
        self.scanning.store(false, Ordering::Release);
    }
}
