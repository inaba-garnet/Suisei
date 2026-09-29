# suisei-dict

読みの推定に使う形態素解析の辞書。build.rs で次のソースを取得して lindera の形式に変換し、バイナリに埋め込む。

| ソース | 版 | ライセンス |
|---|---|---|
| [IPADIC](https://taku910.github.io/mecab/)（lindera が UTF-8 に直した配布物） | mecab-ipadic-2.7.0-20250920 | [licenses/IPADIC-COPYING](licenses/IPADIC-COPYING) |
| [mozcdic-ut-personal-names](https://github.com/utuhiro78/mozcdic-ut-personal-names) | 5e0d754 | Apache License 2.0 |
| [mozcdic-ut-jawiki](https://github.com/utuhiro78/mozcdic-ut-jawiki)（日本語版 Wikipedia の記事名から作られた辞書） | 1f160b9 | [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.ja) |

UT 辞書からは漢字を含む語だけを採り、IPADIC の固有名詞として品詞とコストを付け直している。
変換した辞書は、UT 辞書の jawiki の部分について CC BY-SA 4.0 に従う。
