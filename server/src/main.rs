use std::net::SocketAddr;

use clap::Parser;
use suisei::{AppState, Config, db, scan};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::parse();
    let backup_dir = config.backup_dir();
    let db = db::open(&config.data_dir, &backup_dir)
        .await
        .map_err(std::io::Error::other)?;
    let backup = db::backup::Periodic {
        dir: backup_dir,
        interval: config.backup_interval,
        keep: config.backup_keep as usize,
    };
    tokio::spawn(backup.run(db.clone()));
    let options = scan::Options {
        split_characters: config.split_characters,
    };
    let scanner = scan::Scanner::new(db.clone(), config.music_dir.clone(), options);
    tokio::spawn(scanner.clone().run_periodically(config.scan_interval));
    let state = AppState {
        credentials: config.credentials,
        db,
        scanner,
        now_playing: Default::default(),
        cache_dir: config.data_dir.join("cache"),
        ffmpeg: config.ffmpeg,
        dev: config.dev,
        throttle: Default::default(),
        trust_forwarded_for: config.trust_forwarded_for,
    };
    let listener = TcpListener::bind(config.listen).await?;
    tracing::info!(addr = %config.listen, "listening");
    axum::serve(
        listener,
        // 認証の制限で送り主の IP を使う
        suisei::router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
