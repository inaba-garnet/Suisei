import type { Album, Artist, Song } from '~/utils/subsonic'

/** 検索の結果の種類（docs/web.md の「検索」）。「すべて見る」の URL の `/search/<種類>` に使う。 */
export type SearchKind = 'artists' | 'albums' | 'songs'

export const searchKinds: { kind: SearchKind, label: string, preview: number }[] = [
  { kind: 'artists', label: 'アーティスト', preview: 5 },
  { kind: 'albums', label: 'アルバム', preview: 10 },
  { kind: 'songs', label: '曲', preview: 10 },
]

export function searchKind(value: unknown): SearchKind | undefined {
  return searchKinds.find(k => k.kind === value)?.kind
}

export interface SearchResult {
  artist?: Artist[]
  album?: Album[]
  song?: Song[]
}

/** URL の `q` を検索語にする。前後の空白だけなら検索しない。 */
export function searchQuery(value: unknown): string {
  return typeof value === 'string' ? value.trim() : ''
}
