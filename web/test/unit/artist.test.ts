import { describe, expect, it } from 'vitest'
import { albumsAsAlbumArtist, artistView, groupByAlbum } from '~/utils/artist'

describe('artistView', () => {
  it('知らない値はアルバムアーティストの表示にする', () => {
    expect(artistView('tracks')).toBe('tracks')
    expect(artistView('composer')).toBe('composer')
    expect(artistView(undefined)).toBe('albums')
    expect(artistView('x')).toBe('albums')
  })
})

describe('albumsAsAlbumArtist', () => {
  it('アルバムアーティストになっているアルバムだけを残す', () => {
    const album = (id: string, artists: string[]) => ({ id, name: id, songCount: 1, duration: 1, artists: artists.map(a => ({ id: a, name: a })) })
    const albums = [album('a', ['ar-1']), album('b', ['ar-2']), album('c', ['ar-2', 'ar-1'])]
    expect(albumsAsAlbumArtist(albums, 'ar-1').map(a => a.id)).toEqual(['a', 'c'])
  })
})

describe('groupByAlbum', () => {
  it('続いて並ぶ同じアルバムの曲を一つの見出しにまとめる', () => {
    const song = (id: string, albumId: string) => ({ id, title: id, albumId, album: `${albumId} の名前`, duration: 1 })
    const groups = groupByAlbum([song('1', 'a'), song('2', 'a'), song('3', 'b')])
    expect(groups.map(g => [g.album, g.songs.map(s => s.id)])).toEqual([
      ['a の名前', ['1', '2']],
      ['b の名前', ['3']],
    ])
  })
})
