//! Web のビルド結果を作り直したら、取り込み直す（docs/server.md の「Web クライアントの配信」）。
//! 取り込むファイルが変わっても、ファイルの一覧を読む derive マクロは走り直さないため。

use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let public = Path::new("../web/.output/public");
    // ビルド結果がまだなければ、できたことに気付けるよう web/ を見張る
    let watched = if public.exists() {
        public
    } else {
        Path::new("../web")
    };
    println!("cargo::rerun-if-changed={}", watched.display());
}
