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

    /// トランスコードに使う ffmpeg。見つからなければ元のファイルを返す。
    #[arg(long, env = "SUISEI_FFMPEG", default_value = "ffmpeg")]
    pub ffmpeg: PathBuf,

    /// `キャラクター(CV:声優)` の形のアーティスト名を、キャラクターと声優に分ける。
    #[arg(long, env = "SUISEI_SPLIT_CHARACTERS", default_value_t = true, action = clap::ArgAction::Set)]
    pub split_characters: bool,

    /// 送り主の IP を `X-Forwarded-For` の最後の要素から取る。リバースプロキシの後ろに置くときだけ付ける。
    #[arg(long, env = "SUISEI_TRUST_FORWARDED_FOR", default_value_t = false, action = clap::ArgAction::Set)]
    pub trust_forwarded_for: bool,

    /// DB のバックアップの置き場所。既定はデータの置き場所の下の `backup`。
    #[arg(long, env = "SUISEI_BACKUP_DIR")]
    pub backup_dir: Option<PathBuf>,

    /// DB のバックアップの間隔（`1d`、`12h` など）。最新の写しの時刻から数える。`0` で定期の写しを止める。
    #[arg(long, env = "SUISEI_BACKUP_INTERVAL", default_value = "1d", value_parser = humantime::parse_duration)]
    pub backup_interval: Duration,

    /// 残す定期の写しの数。`0` で定期の写しを止める。
    #[arg(long, env = "SUISEI_BACKUP_KEEP", default_value_t = 7)]
    pub backup_keep: u32,

    /// Spotify のアプリの Client ID。渡したときだけ Spotify 連携を使える（docs/spotify.md）。
    #[arg(long, env = "SUISEI_SPOTIFY_CLIENT_ID")]
    pub spotify_client_id: Option<String>,

    /// 開発モード。Web クライアントが未完成の UI を出す。
    #[arg(long, env = "SUISEI_DEV", default_value_t = false, action = clap::ArgAction::Set)]
    pub dev: bool,

    #[command(flatten)]
    pub credentials: Credentials,
}

impl Config {
    pub fn backup_dir(&self) -> PathBuf {
        self.backup_dir
            .clone()
            .unwrap_or_else(|| self.data_dir.join("backup"))
    }
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn parse(args: &[&str]) -> Config {
        let base = [
            "suisei",
            "--music-dir",
            "m",
            "--user",
            "u",
            "--password",
            "p",
        ];
        Config::try_parse_from(base.iter().chain(args)).unwrap()
    }

    #[test]
    fn characters_are_split_by_default() {
        assert!(parse(&[]).split_characters);
        assert!(!parse(&["--split-characters", "false"]).split_characters);
    }

    #[test]
    fn backups_default_to_data_dir() {
        let config = parse(&["--data-dir", "d"]);
        assert_eq!(config.backup_dir(), PathBuf::from("d/backup"));
        assert_eq!(config.backup_interval, Duration::from_secs(24 * 60 * 60));
        assert_eq!(config.backup_keep, 7);
        assert_eq!(
            parse(&["--backup-dir", "b"]).backup_dir(),
            PathBuf::from("b")
        );
        assert_eq!(parse(&["--backup-keep", "0"]).backup_keep, 0);
    }

    #[test]
    fn dev_is_off_by_default() {
        assert!(!parse(&[]).dev);
        assert!(parse(&["--dev", "true"]).dev);
    }
}
