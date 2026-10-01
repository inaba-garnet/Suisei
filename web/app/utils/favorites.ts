/** お気に入りの曲の並び順（docs/web.md の「一覧」）。 */
import type { TrackSort } from '~/utils/tracks'

export type FavoriteSort = 'newest' | 'oldest' | TrackSort

export const favoriteSortOptions: { value: FavoriteSort, label: string }[] = [
  { value: 'newest', label: '新しい順' },
  { value: 'oldest', label: '古い順' },
  { value: 'title', label: 'タイトル順' },
  { value: 'artist', label: 'アーティスト順' },
  { value: 'album', label: 'アルバム順' },
]

export function favoriteSort(value: unknown): FavoriteSort {
  return favoriteSortOptions.find(option => option.value === value)?.value ?? 'newest'
}

/** サーバーの `songSort` に渡す値。新しい順と古い順はサーバーの既定（新しい順）を使う。 */
export function favoriteSongSort(sort: FavoriteSort): TrackSort | undefined {
  return sort === 'newest' || sort === 'oldest' ? undefined : sort
}
