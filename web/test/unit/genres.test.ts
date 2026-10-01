import { describe, expect, it } from 'vitest'
import { genreHues, genreSort, sortGenres } from '~/utils/genres'

describe('sortGenres', () => {
  const genre = (value: string, songCount: number) => ({ value, songCount, albumCount: 1 })
  const genres = [genre('ロック', 3), genre('アニソン', 5), genre('Pop', 3)]

  it('名前順は読みの順にする', () => {
    expect(sortGenres(genres, 'name').map(g => g.value)).toEqual(['Pop', 'アニソン', 'ロック'])
  })

  it('曲の多い順で同じ曲数なら名前順にする', () => {
    expect(sortGenres(genres, 'songs').map(g => g.value)).toEqual(['アニソン', 'Pop', 'ロック'])
  })

  it('知らない並び順は名前順にする', () => {
    expect(genreSort('songs')).toBe('songs')
    expect(genreSort('x')).toBe('name')
  })
})

describe('genreHues', () => {
  it('名前順で隣り合うジャンルには違う色を割り当て、並び順を変えても色は変えない', () => {
    const genre = (value: string, songCount: number) => ({ value, songCount, albumCount: 1 })
    const hues = genreHues([genre('ロック', 3), genre('アニソン', 5), genre('Pop', 3)])
    expect(hues.get('Pop')).not.toBe(hues.get('アニソン'))
    expect(hues.get('アニソン')).not.toBe(hues.get('ロック'))
    expect(genreHues([genre('アニソン', 5), genre('ロック', 3), genre('Pop', 3)])).toEqual(hues)
  })
})
