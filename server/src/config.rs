use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// 起動引数と環境変数から読む設定。
#[derive(Debug, Clone, clap::Parser)]
#[command(version, about)]
pub struct Config {
    #[arg(long, env = "SUISEI_LISTEN", default_value = "0.0.0.0:4533")]
    pub listen: SocketAddr,

    /// DB などを置くディレクトリ。
    #[arg(long, env = "SUISEI_DATA_DIR", default_value = "data")]
    pub data_dir: PathBuf,

    /// 音楽フォルダ。一つだけ渡す。
    #[arg(long, env = "SUISEI_MUSIC_DIR")]
    pub music_dir: PathBuf,

    /// 定期スキャンの間隔（`1h`、`30m` など）。前のスキャンが終わってから数える。`0` で起動時の一度だけ。
    #[arg(long, env = "SUISEI_SCAN_INTERVAL", default_value = "1h", value_parser = humantime::parse_duration)]
    pub scan_interval: Duration,

    #[command(flatten)]
    pub credentials: Credentials,
}

/// 一人で使う前提なので、利用者は一人分だけ持つ。
#[derive(Debug, Clone, clap::Args)]
pub struct Credentials {
    #[arg(long, env = "SUISEI_USER")]
    pub user: String,

    #[arg(long, env = "SUISEI_PASSWORD", hide_env_values = true)]
    pub password: String,

    /// 指定したときだけ OpenSubsonic の apiKey 認証を受け付ける。
    #[arg(long, env = "SUISEI_API_KEY", hide_env_values = true)]
    pub api_key: Option<String>,
}
