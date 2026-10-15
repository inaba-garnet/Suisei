/** 読みの出どころ（docs/schema.md の「日本語の並べ替え」）。 */
export type ReadingSource = 'manual' | 'sort_tag' | 'name' | 'estimated'

/** `GET`、`PUT /api/readings/{id}` が返す一件。 */
export interface ReadingEntry {
  id: string
  name: string
  /** アルバムと曲の表示用アーティスト */
  artist: string | null
  /** 手動の読みがあればそれ、なければ自動の読み */
  reading: string | null
  source: ReadingSource | null
  /** スキャンで得た自動の読み。手動の読みを外すとこれに戻る */
  autoReading: string | null
  autoSource: ReadingSource | null
}

const SOURCE_LABELS: Record<ReadingSource, string> = {
  manual: '手動',
  sort_tag: 'ソート用タグ',
  name: '名前',
  estimated: '推定',
}

/** 出どころの表示名。読みがなければ「読みなし」。 */
export function sourceLabel(source: ReadingSource | null): string {
  return source ? SOURCE_LABELS[source] : '読みなし'
}
