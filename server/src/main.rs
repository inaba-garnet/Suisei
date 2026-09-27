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
    let db = db::open(&config.data_dir)
        .await
        .map_err(std::io::Error::other)?;
    // 定期実行と startScan は #3 の 2 番目の PR で入れる。それまでは起動時に一度だけ走らせる
    tokio::spawn(startup_scan(db.clone(), config.music_dir.clone()));
    let state = AppState {
        credentials: config.credentials,
        db,
    };
    let listener = TcpListener::bind(config.listen).await?;
    tracing::info!(addr = %config.listen, "listening");
    axum::serve(listener, suisei::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn startup_scan(db: db::Pool, music_dir: std::path::PathBuf) {
    let started = std::time::Instant::now();
    match scan::run(&db, &music_dir, scan::Mode::Quick).await {
        Ok(summary) => tracing::info!(
            files = summary.files,
            read = summary.read,
            failed = summary.failed,
            tracks = summary.tracks,
            albums = summary.albums,
            artists = summary.artists,
            elapsed_ms = started.elapsed().as_millis(),
            "scan finished"
        ),
        Err(err) => tracing::error!(error = %err, "scan failed"),
    }
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
