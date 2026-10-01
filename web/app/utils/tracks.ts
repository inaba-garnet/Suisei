/** トラックの一覧の並び順（docs/web.md の「一覧」）。値はサーバーの `search3` の `songSort` に渡す。 */
export type TrackSort = 'title' | 'album' | 'artist'

export const trackSortOptions: { value: TrackSort, label: string }[] = [
  { value: 'title', label: '曲名順' },
  { value: 'album', label: 'アルバム順' },
  { value: 'artist', label: 'アーティスト順' },
]

export function trackSort(value: unknown): TrackSort {
  return value === 'album' || value === 'artist' ? value : 'title'
}
