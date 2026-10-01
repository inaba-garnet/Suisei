import { describe, expect, it } from 'vitest'
import { favoriteSongSort, favoriteSort } from '~/utils/favorites'

describe('favoriteSort', () => {
  it('知らない並び順は新しい順にする', () => {
    expect(favoriteSort('album')).toBe('album')
    expect(favoriteSort('x')).toBe('newest')
    expect(favoriteSort(undefined)).toBe('newest')
  })

  it('新しい順と古い順はサーバーの既定に任せ、それ以外は songSort に渡す', () => {
    expect(favoriteSongSort('newest')).toBeUndefined()
    expect(favoriteSongSort('oldest')).toBeUndefined()
    expect(favoriteSongSort('artist')).toBe('artist')
  })
})
