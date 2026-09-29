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
    };
    let listener = TcpListener::bind(config.listen).await?;
    tracing::info!(addr = %config.listen, "listening");
    axum::serve(listener, suisei::router(state))
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
