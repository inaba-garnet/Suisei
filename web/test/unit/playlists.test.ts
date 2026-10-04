import type { Song } from '~/utils/subsonic'
import { describe, expect, it } from 'vitest'
import { playlistUpdate } from '~/utils/playlists'

const song = (id: string) => ({ id, title: id, duration: 1 }) as Song
const songs = [song('a'), song('b'), song('a')]
const rows = (keys: number[]) => keys.map(key => ({ key, song: songs[key]! }))

describe('playlistUpdate', () => {
  it('何も変えていなければ呼ばない', () => {
    expect(playlistUpdate('pl', 'x', { name: 'x', songs }, rows([0, 1, 2]))).toBeUndefined()
  })

  it('名前だけを変えたら名前だけを送る', () => {
    expect(playlistUpdate('pl', ' y ', { name: 'x', songs }, rows([0, 1, 2]))).toEqual({ playlistId: 'pl', name: 'y' })
  })

  it('空の名前は送らない', () => {
    expect(playlistUpdate('pl', ' ', { name: 'x', songs }, rows([0, 1, 2]))).toBeUndefined()
  })

  it('並べ替えたら、元の曲をすべて外して新しい並びで入れ直す', () => {
    expect(playlistUpdate('pl', 'x', { name: 'x', songs }, rows([2, 0, 1]))).toEqual({
      playlistId: 'pl',
      songIndexToRemove: [0, 1, 2],
      songIdToAdd: ['a', 'a', 'b'],
    })
  })

  it('同じ曲が二度入っていても、外した方だけを除く', () => {
    expect(playlistUpdate('pl', 'x', { name: 'x', songs }, rows([0, 1]))).toEqual({
      playlistId: 'pl',
      songIndexToRemove: [0, 1, 2],
      songIdToAdd: ['a', 'b'],
    })
  })
})
