/** アルバムの一覧の並び順と表示（docs/web.md の「一覧」）。 */
export type AlbumSort = 'name' | 'newest' | 'artist' | 'year'

export const albumSortOptions: { value: AlbumSort, label: string }[] = [
  { value: 'name', label: '名前順' },
  { value: 'newest', label: '新着順' },
  { value: 'artist', label: 'アーティスト順' },
  { value: 'year', label: '年順' },
]

export function albumSort(value: unknown): AlbumSort {
  return albumSortOptions.find(option => option.value === value)?.value ?? 'name'
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
  }
}

export type AlbumView = 'grid' | 'list'

export function albumView(value: unknown): AlbumView {
  return value === 'list' ? 'list' : 'grid'
}
