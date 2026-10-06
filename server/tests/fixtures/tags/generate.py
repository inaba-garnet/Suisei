#!/usr/bin/env python3
"""タグの読み取りのテストに使う、無音の音声ファイルを作る。

ffmpeg で 1 秒の無音を作り、mutagen でタグを書く。
読み取り側（lofty）と別のライブラリで書き、読み違いを書き込み側で打ち消さないようにする。

使い方: python3 generate.py（ffmpeg と mutagen が要る）
"""

import pathlib
import struct
import subprocess
import zlib

from mutagen.flac import FLAC, Picture
from mutagen.id3 import (
    ID3,
    TALB,
    TCMP,
    TCON,
    TDAT,
    TDRC,
    TIT2,
    TPE1,
    TPE2,
    TPOS,
    TRCK,
    TSO2,
    TSOA,
    TSOP,
    TSOT,
    TSRC,
    TXXX,
    TYER,
)
from mutagen.mp4 import MP4, MP4FreeForm

OUT = pathlib.Path(__file__).resolve().parent

# 全項目を書くファイルの値。テスト（src/tags/raw.rs）の期待値と合わせる
TITLE = "テスト曲"
TITLE_SORT = "てすときょく"
ARTIST = "歌手A feat. 歌手B"
ARTISTS = ["歌手A", "歌手B"]
ARTIST_SORT = "Kashu A feat. Kashu B"
ALBUM = "テストアルバム"
ALBUM_SORT = "てすとあるばむ"
ALBUM_ARTIST = "歌手A"
ALBUM_ARTISTS = ["歌手A"]
ALBUM_ARTIST_SORT = "かしゅA"
MB_ARTIST_IDS = [
    "00000000-0000-0000-0000-00000000000a",
    "00000000-0000-0000-0000-00000000000b",
]
MB_ALBUM_ARTIST_IDS = ["00000000-0000-0000-0000-00000000000a"]
DATE = "2015-09-23"
GENRES = ["Rock", "Pop"]
ISRC = "JPXX01500001"


def silence(path: pathlib.Path, codec_args: list[str]) -> None:
    subprocess.run(
        [
            "ffmpeg", "-y", "-loglevel", "error",
            "-f", "lavfi", "-i", "anullsrc=r=8000:cl=mono", "-t", "1",
            "-map_metadata", "-1", "-fflags", "+bitexact", "-flags:a", "+bitexact",
            *codec_args, str(path),
        ],
        check=True,
    )


def flac_full() -> None:
    path = OUT / "full.flac"
    silence(path, ["-c:a", "flac"])
    f = FLAC(path)
    f.delete()
    f["TITLE"] = TITLE
    f["TITLESORT"] = TITLE_SORT
    f["ARTIST"] = ARTIST
    f["ARTISTS"] = ARTISTS
    f["ARTISTSORT"] = ARTIST_SORT
    f["ALBUM"] = ALBUM
    f["ALBUMSORT"] = ALBUM_SORT
    f["ALBUMARTIST"] = ALBUM_ARTIST
    f["ALBUMARTISTS"] = ALBUM_ARTISTS
    f["ALBUMARTISTSORT"] = ALBUM_ARTIST_SORT
    f["MUSICBRAINZ_ARTISTID"] = MB_ARTIST_IDS
    f["MUSICBRAINZ_ALBUMARTISTID"] = MB_ALBUM_ARTIST_IDS
    f["DISCNUMBER"] = "2"
    f["TRACKNUMBER"] = "3"
    f["DATE"] = DATE
    f["GENRE"] = GENRES
    f["COMPILATION"] = "1"
    f["ISRC"] = ISRC
    f.save()


def flac_empty() -> None:
    path = OUT / "notag.flac"
    silence(path, ["-c:a", "flac"])
    f = FLAC(path)
    f.delete()
    f.save()


def make_png() -> bytes:
    """1x1 の不透明な白の PNG。埋め込みとフォルダの画像のテストに使う。"""

    def chunk(kind: bytes, body: bytes) -> bytes:
        crc = zlib.crc32(kind + body) & 0xFFFFFFFF
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", crc)

    header = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    pixels = zlib.compress(b"\x00\xff\xff\xff")
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", pixels) + chunk(b"IEND", b"")


PNG = make_png()


def png() -> None:
    (OUT / "cover.png").write_bytes(PNG)


def flac_picture() -> None:
    path = OUT / "picture.flac"
    silence(path, ["-c:a", "flac"])
    f = FLAC(path)
    f.delete()
    f["TITLE"] = "画像つき"
    f["ALBUM"] = "画像のアルバム"
    picture = Picture()
    picture.type = 3  # 表紙
    picture.mime = "image/png"
    picture.width = picture.height = 1
    picture.depth = 24
    picture.data = PNG
    f.add_picture(picture)
    f.save()


def id3_frames() -> list:
    return [
        TIT2(encoding=3, text=TITLE),
        TSOT(encoding=3, text=TITLE_SORT),
        TPE1(encoding=3, text=ARTIST),
        TXXX(encoding=3, desc="ARTISTS", text=ARTISTS),
        TSOP(encoding=3, text=ARTIST_SORT),
        TALB(encoding=3, text=ALBUM),
        TSOA(encoding=3, text=ALBUM_SORT),
        TPE2(encoding=3, text=ALBUM_ARTIST),
        TXXX(encoding=3, desc="ALBUMARTISTS", text=ALBUM_ARTISTS),
        TSO2(encoding=3, text=ALBUM_ARTIST_SORT),
        TXXX(encoding=3, desc="MusicBrainz Artist Id", text=MB_ARTIST_IDS),
        TXXX(encoding=3, desc="MusicBrainz Album Artist Id", text=MB_ALBUM_ARTIST_IDS),
        TPOS(encoding=3, text="2/2"),
        TRCK(encoding=3, text="3/10"),
        TDRC(encoding=3, text=DATE),
        TCON(encoding=3, text=GENRES),
        TCMP(encoding=3, text="1"),
        TSRC(encoding=3, text=ISRC),
    ]


def mp3_full(name: str, version: int) -> None:
    path = OUT / name
    silence(path, ["-c:a", "libmp3lame", "-b:a", "8k", "-write_xing", "0", "-id3v2_version", "0"])
    tags = ID3()
    for frame in id3_frames():
        tags.add(frame)
    if version == 3:
        # ID3v2.3 に TDRC はないので、年と日付に分ける。
        # mutagen の update_to_v23 は iTunes が v2.3 にも書くソート用のフレームを消すので使わない
        year, month, day = DATE.split("-")
        tags.delall("TDRC")
        tags.add(TYER(encoding=3, text=year))
        tags.add(TDAT(encoding=3, text=day + month))
    # ID3v2.3 は複数値を持てないので、mutagen の既定（Picard と同じ）どおり "/" でつなぐ
    tags.save(path, v2_version=version, v1=0)


def mp3_id3v1() -> None:
    path = OUT / "id3v1.mp3"
    silence(path, ["-c:a", "libmp3lame", "-b:a", "8k", "-write_xing", "0", "-id3v2_version", "0"])
    tags = ID3()
    tags.add(TIT2(encoding=0, text="Title"))
    tags.add(TPE1(encoding=0, text="Artist"))
    tags.add(TALB(encoding=0, text="Album"))
    tags.add(TRCK(encoding=0, text="3"))
    tags.add(TDRC(encoding=0, text="2015"))
    # v1 だけを残す
    tags.save(path, v1=2)
    ID3(path).delete(path, delete_v1=False, delete_v2=True)


def m4a_full() -> None:
    path = OUT / "full.m4a"
    silence(path, ["-c:a", "aac", "-b:a", "16k"])
    f = MP4(path)
    f.delete()

    def freeform(values: list[str]) -> list:
        return [MP4FreeForm(v.encode()) for v in values]

    f["\xa9nam"] = TITLE
    f["sonm"] = TITLE_SORT
    f["\xa9ART"] = ARTIST
    f["----:com.apple.iTunes:ARTISTS"] = freeform(ARTISTS)
    f["soar"] = ARTIST_SORT
    f["\xa9alb"] = ALBUM
    f["soal"] = ALBUM_SORT
    f["aART"] = ALBUM_ARTIST
    f["----:com.apple.iTunes:ALBUMARTISTS"] = freeform(ALBUM_ARTISTS)
    f["soaa"] = ALBUM_ARTIST_SORT
    f["----:com.apple.iTunes:MusicBrainz Artist Id"] = freeform(MB_ARTIST_IDS)
    f["----:com.apple.iTunes:MusicBrainz Album Artist Id"] = freeform(MB_ALBUM_ARTIST_IDS)
    f["disk"] = [(2, 2)]
    f["trkn"] = [(3, 10)]
    f["\xa9day"] = DATE
    f["\xa9gen"] = GENRES
    f["cpil"] = True
    f["----:com.apple.iTunes:ISRC"] = freeform([ISRC])
    f.save()


if __name__ == "__main__":
    flac_full()
    flac_empty()
    mp3_full("full.mp3", 4)
    mp3_full("full-v23.mp3", 3)
    mp3_id3v1()
    m4a_full()
    flac_picture()
    png()
