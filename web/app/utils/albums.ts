/** アルバムの一覧の並び順と表示（docs/web.md の「一覧」）。 */
export type AlbumSort = 'name' | 'newest' | 'artist' | 'year' | 'recent' | 'frequent'

export const albumSortOptions: { value: AlbumSort, label: string }[] = [
  { value: 'name', label: '名前順' },
  { value: 'newest', label: '新着順' },
  { value: 'artist', label: 'アーティスト順' },
  { value: 'year', label: '年順' },
  { value: 'recent', label: '最近再生した順' },
  { value: 'frequent', label: 'よく聴く順' },
]

export function albumSort(value: unknown): AlbumSort {
  return parseAlbumSort(value) ?? 'name'
}

/** 知っている並び順なら返す。URL の `?sort=` を読むのに使う。 */
export function parseAlbumSort(value: unknown): AlbumSort | undefined {
  return albumSortOptions.find(option => option.value === value)?.value
}

/** `getAlbumList2` に渡す並べ方。年順は新しい年から並べるので、範囲を大きい年から書く。 */
export function albumListParams(sort: AlbumSort): Record<string, string | number> {
  switch (sort) {
    case 'name':
      return { type: 'alphabeticalByName' }
    case 'newest':
      return { type: 'newest' }
    case 'artist':
      return { type: 'alphabeticalByArtist' }
    case 'year':
      return { type: 'byYear', fromYear: 9999, toYear: 0 }
    // 再生したことのないアルバムは含まない（docs/schema.md）
    case 'recent':
      return { type: 'recent' }
    case 'frequent':
      return { type: 'frequent' }
  }
}

export type AlbumView = 'grid' | 'list'

export function albumView(value: unknown): AlbumView {
  return value === 'list' ? 'list' : 'grid'
}
