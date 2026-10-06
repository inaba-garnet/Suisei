//! スキャンを一度に一つだけ走らせ、`getScanStatus` に返す状態を持つ。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard, watch};

use super::{Mode, Options, run_with_progress};
use crate::db::Pool;
use crate::settings::Settings;

#[derive(Debug)]
pub struct Scanner {
    pool: Pool,
    music_dir: PathBuf,
    /// 設定の画面で変わるので、スキャンのたびに読む
    options: Mutex<Options>,
    /// スキャン中は取られている
    lock: Arc<AsyncMutex<()>>,
    scanning: AtomicBool,
    /// スキャン中に見つけた音声の数
    progress: Arc<AtomicUsize>,
    last: Mutex<Option<Finished>>,
    /// 成功したスキャンの数。スキャンの後の処理（Spotify の対応の付け直しなど）を起こす
    finished: watch::Sender<u64>,
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
    pub fn new(pool: Pool, music_dir: PathBuf, options: Options) -> Arc<Self> {
        Arc::new(Self {
            pool,
            music_dir,
            options: Mutex::new(options),
            lock: Arc::default(),
            scanning: AtomicBool::new(false),
            progress: Arc::default(),
            last: Mutex::new(None),
            finished: watch::Sender::new(0),
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

    /// 次のスキャンから使うタグの解釈を変える。
    pub fn set_options(&self, options: Options) {
        *self.options.lock().expect("設定のロックが壊れた") = options;
    }

    /// すぐに一度スキャンし、設定の間隔ごとに繰り返す。間隔は前のスキャンが終わってから数える。
    /// 間隔が 0 なら、設定が変わるまで待つ。手動のスキャンが走っていれば、その回は飛ばす。
    /// 設定が変われば、タグの解釈を差し替え、新しい間隔で次の時刻を数え直す。
    pub async fn run_periodically(self: Arc<Self>, mut settings: watch::Receiver<Settings>) {
        loop {
            self.set_options(settings.borrow_and_update().tag_options());
            if let Some(guard) = self.begin() {
                self.scan(Mode::Quick, guard).await;
            }
            let finished = Instant::now();
            loop {
                let interval = settings.borrow().scan_interval;
                let wait = async {
                    if interval.is_zero() {
                        std::future::pending::<()>().await;
                    }
                    tokio::time::sleep_until((finished + interval).into()).await;
                };
                tokio::select! {
                    () = wait => break,
                    changed = settings.changed() => {
                        if changed.is_err() {
                            return;
                        }
                        self.set_options(settings.borrow_and_update().tag_options());
                    }
                }
            }
        }
    }

    /// スキャンが成功するたびに値が変わる受け口。
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.finished.subscribe()
    }

    pub fn music_dir(&self) -> &Path {
        &self.music_dir
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
        let options = *self.options.lock().expect("設定のロックが壊れた");
        let result = run_with_progress(
            &self.pool,
            &self.music_dir,
            mode,
            options,
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
                    load_ms = summary.timings.load_ms,
                    collect_ms = summary.timings.collect_ms,
                    build_ms = summary.timings.build_ms,
                    write_ms = summary.timings.write_ms,
                    "scan finished"
                );
                *self.last.lock().expect("状態のロックが壊れた") = Some(Finished {
                    at: SystemTime::now(),
                    files: summary.files,
                    folders: summary.folders,
                });
                self.finished.send_modify(|n| *n += 1);
            }
            Err(err) => tracing::error!(?mode, error = %err, "scan failed"),
        }
        self.scanning.store(false, Ordering::Release);
    }
}
