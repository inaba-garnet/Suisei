/** ジャンルの一覧の並び順（docs/web.md の「一覧」）。 */
import type { Genre } from '~/utils/subsonic'

export type GenreSort = 'name' | 'songs'

export const genreSortOptions: { value: GenreSort, label: string }[] = [
  { value: 'name', label: '名前順' },
  { value: 'songs', label: '曲の多い順' },
]

export function genreSort(value: unknown): GenreSort {
  return value === 'songs' ? value : 'name'
}

const collator = new Intl.Collator('ja')

/** 名前順は読みの順。曲の多い順で同じ曲数なら名前順。 */
export function sortGenres(genres: Genre[], sort: GenreSort): Genre[] {
  return [...genres].sort((a, b) =>
    (sort === 'songs' ? b.songCount - a.songCount : 0) || collator.compare(a.value, b.value))
}

/** タイルの色相。アクセントの青に合う色から、隣り合っても見分けやすいものを選んでいる。 */
const HUES = [230, 295, 15, 170, 265, 330, 40, 200]

/**
 * ジャンルごとのタイルの色相（0〜359）。名前順に並べたときの位置で色を順に割り当てる。
 * 名前から計算すると似た色が並びやすいので、名前順では隣り合うタイルがいつも違う色になるようにする。
 */
export function genreHues(genres: Genre[]): Map<string, number> {
  return new Map(sortGenres(genres, 'name').map((genre, i) => [genre.value, HUES[i % HUES.length]!]))
}
