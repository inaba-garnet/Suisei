//! 形態素解析の辞書を作る（docs/server.md の「形態素解析」）。
//! IPADIC に Mozc UT 辞書の漢字を含む語を固有名詞として足し、lindera の形式に変換する。
//! 取得したファイルは `SUISEI_DICT_CACHE`（なければ `OUT_DIR`）に置き、ハッシュが合えば取得し直さない。

use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::metadata::Metadata;
use sha2::{Digest, Sha256};

struct Source {
    file: &'static str,
    url: &'static str,
    sha256: &'static str,
}

/// UTF-8 に直した IPADIC（lindera の配布物）。
const IPADIC: Source = Source {
    file: "mecab-ipadic-2.7.0-20250920.tar.gz",
    url: "https://lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz",
    sha256: "a7ba9f645ffe7094e56ae1c4a81d100df8fbb1e28bbe1792622e9728e162db3d",
};

/// Mozc UT 辞書。先に並べたものの読みを優先する。
const UT: [Source; 2] = [
    Source {
        file: "mozcdic-ut-personal-names.txt.bz2",
        url: "https://raw.githubusercontent.com/utuhiro78/mozcdic-ut-personal-names/5e0d754f3162c3e1d83568307da2c9d3ea54731f/mozcdic-ut-personal-names.txt.bz2",
        sha256: "568ce565cc56c51a6ba22083faddd9b96215a6037fdab8685d85d0be5379ebd1",
    },
    Source {
        file: "mozcdic-ut-jawiki.txt.bz2",
        url: "https://raw.githubusercontent.com/utuhiro78/mozcdic-ut-jawiki/1f160b92bee490d2f3b8a20399e4d1e63d25c67d/mozcdic-ut-jawiki.txt.bz2",
        sha256: "87953b2402ef87d91568cad648387dee4982adbc33315fd8d43a3c83190e894c",
    },
];

/// IPADIC の「名詞,固有名詞,一般」の文脈 ID。
const PROPER_NOUN_ID: u32 = 1288;

/// lindera-ipadic の metadata.json と同じ。
const METADATA: &str = r#"{
  "name": "suisei",
  "encoding": "UTF-8",
  "default_word_cost": -10000,
  "default_left_context_id": 0,
  "default_right_context_id": 0,
  "default_field_value": "*",
  "flexible_csv": true,
  "skip_invalid_cost_or_id": false,
  "normalize_details": true,
  "dictionary_schema": {
    "fields": [
      "surface", "left_context_id", "right_context_id", "cost",
      "part_of_speech", "part_of_speech_subcategory_1", "part_of_speech_subcategory_2",
      "part_of_speech_subcategory_3", "conjugation_form", "conjugation_type",
      "base_form", "reading", "pronunciation"
    ]
  },
  "user_dictionary_schema": {
    "fields": ["surface", "part_of_speech", "reading"]
  }
}"#;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=SUISEI_DICT_CACHE");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    println!("cargo::rustc-env=LINDERA_WORKDIR={}", out_dir.display());

    let sources_dir = std::env::var_os("SUISEI_DICT_CACHE")
        .map_or_else(|| out_dir.join("sources"), PathBuf::from);
    fs::create_dir_all(&sources_dir)?;
    let input_dir = out_dir.join("input");
    if input_dir.exists() {
        fs::remove_dir_all(&input_dir)?;
    }
    fs::create_dir_all(&input_dir)?;

    let ipadic = fetch(&IPADIC, &sources_dir)?;
    extract_ipadic(&ipadic, &input_dir)?;
    let mut ut = Vec::new();
    for source in &UT {
        ut.push(fetch(source, &sources_dir)?);
    }
    write_ut(&ut, &input_dir.join("ut.csv"))?;

    let metadata: Metadata = serde_json::from_str(METADATA)?;
    DictionaryBuilder::new(metadata).build_dictionary(&input_dir, &out_dir.join("suisei"))?;
    fs::remove_dir_all(&input_dir)?;
    Ok(())
}

/// 取得済みでハッシュが合えば使い回し、なければ取得する。
fn fetch(source: &Source, dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join(source.file);
    if path.exists() && sha256(&fs::read(&path)?) == source.sha256 {
        return Ok(path);
    }
    let bytes = ureq::get(source.url)
        .call()?
        .into_body()
        .with_config()
        .limit(256 * 1024 * 1024)
        .read_to_vec()?;
    let actual = sha256(&bytes);
    if actual != source.sha256 {
        return Err(format!("{} のハッシュが合わない: {actual}", source.file).into());
    }
    fs::write(&path, bytes)?;
    Ok(path)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// アーカイブの最上位のディレクトリを外して展開する。
fn extract_ipadic(archive: &Path, dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(fs::File::open(archive)?));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let Some(name) = path.file_name() else {
            continue;
        };
        let name = name.to_string_lossy();
        // 変換に使うのは語彙の CSV と定義ファイルだけ
        if name.ends_with(".csv") || name.ends_with(".def") {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            fs::write(dir.join(&*name), bytes)?;
        }
    }
    Ok(())
}

/// UT 辞書の漢字を含む語を、IPADIC の固有名詞の行にする。
/// 長い語ほど小さいコストにする。同じコストだと、短い語に分けたほうが安くなるため。
fn write_ut(files: &[PathBuf], out: &Path) -> Result<(), Box<dyn Error>> {
    let mut seen = HashSet::new();
    let mut csv = std::io::BufWriter::new(fs::File::create(out)?);
    for file in files {
        let reader = BufReader::new(bzip2::read::BzDecoder::new(fs::File::open(file)?));
        for line in reader.lines() {
            let line = line?;
            // 読み、左文脈 ID、右文脈 ID、コスト、表記
            let fields: Vec<&str> = line.split('\t').collect();
            let [reading, _, _, _, surface] = fields[..] else {
                continue;
            };
            if !surface.chars().any(is_kanji)
                || surface.contains([',', '"'])
                || !reading.chars().all(|c| is_hiragana(c) || c == 'ー')
                || !seen.insert(surface.to_owned())
            {
                continue;
            }
            let reading: String = reading.chars().map(to_katakana).collect();
            let length = i32::try_from(surface.chars().count()).unwrap_or(i32::MAX);
            let cost = 2000_i32
                .saturating_sub(length.saturating_mul(2500))
                .max(-32000);
            writeln!(
                csv,
                "{surface},{PROPER_NOUN_ID},{PROPER_NOUN_ID},{cost},名詞,固有名詞,一般,*,*,*,{surface},{reading},{reading}"
            )?;
        }
    }
    csv.flush()?;
    Ok(())
}

fn is_kanji(c: char) -> bool {
    matches!(c, '\u{3005}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{3134f}')
}

fn is_hiragana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}')
}

fn to_katakana(c: char) -> char {
    if is_hiragana(c) {
        char::from_u32(c as u32 + 0x60).unwrap_or(c)
    } else {
        c
    }
}
